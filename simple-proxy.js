const Shell = require('child_process');
const Net = require('net');
const Crypto = require('crypto');
const Path = require('path');
const Fs = require('fs');
const NumberFormat = new Intl.NumberFormat();

const APP_NAME = 'Simple Proxy v1.2.0';
const ZEROES = '000000';
const INSTANCE_ID = (ZEROES + Crypto.randomBytes(4).readUIntBE(0, 4) % 10000).slice(-4);
const IDLE_TIMEOUT = 1800 * 1000;   // 30 minutes
var stopping = false;

const consoleLog = (message) => {
    console.log(`${getLocalTimestamp()} [${INSTANCE_ID}] ${message}`);
};

const getLocalTimestamp = () => {
    const d = new Date();
    const UTC2Local = new Date(d.getTime() - d.getTimezoneOffset() * 60 * 1000).toISOString();
    return UTC2Local.replace('T', ' ').replace('Z', '');
}

async function getVmIpAddress(command, tag, index) {
    return new Promise((resolve, reject) => {
        consoleLog(`Resolving target IP adddress using command: "${command} (${tag})", index: ${index}`);
        Shell.exec(command, (error, stdout, stderr) => {
            if (error) {
                consoleLog(error.message.split('\n')[0]);
            } 
            if (stdout) {
                var entries = stdout.split('\n').filter(v => v.toString().trim().startsWith(tag));
                if (entries.length > 0) {
                    var ip = entries[0].trim().split(' ').filter(v => v.length > 0)[index];                
                    resolve([true, ip]);
                } else {
                    resolve([false, 'Unable to resolve IP address. Is VM started?']);
                }
            } else {
                resolve([false, stderr.split('\n')[0]]);
            }
        });
    });
}

const isAuthorized = (remoteIp, authorized) => {
    if (authorized.includes(remoteIp)) {
        return true;
    }
    for (i in authorized) {
        const s = authorized[i];
        if (s.endsWith('.*')) {
            const prefix = s.substring(0, s.lastIndexOf('.'));
            if (remoteIp.startsWith(prefix+'.')) {
                return true;
            }
        }
    }
    return false;
}

const forwardPort = (sourcePort, targetIp, targetPort, authorized, restart) => {
    const connections = new Map();
    // Setup TCP socket server
    const server = new Net.Server();
    server.listen(sourcePort, '0.0.0.0', () => {
        consoleLog(`Forwarding port-${sourcePort} to ${targetIp}:${targetPort}`);
    });
    server.on('error', (e) => {
        if ('EADDRINUSE' == e.code) {
            consoleLog(`Port ${sourcePort} is already used`);
        } else {
            consoleLog(`${e}`);
        }
    });
    // Graceful shutdown
    const gracefulShutdown = () => {
        for (const k of connections.keys()) {
            consoleLog(`Stopping ${k}`);
            connections.get(k).end();
        }
        server.close(() => {
            consoleLog(`Proxy service port-${targetPort} stopped`);
        });
    };
    process.on('SIGTERM', () => {
        if (!stopping) {
            stopping = true;
            consoleLog('Kill signal detected');
        }
        gracefulShutdown();
    });
    process.on('SIGINT', () => {
        if (!stopping) {
            stopping = true;
            consoleLog('Control-C detected');
        }
        gracefulShutdown();
    });
    // Start TCP socket server
    server.on('connection', (socket) => {
        // check remote address
        const remoteIp = socket.remoteAddress;
        if (!isAuthorized(remoteIp, authorized)) {
            consoleLog(`Unknown caller ${remoteIp} connection to ${targetPort} rejected`);
            socket.destroy();
            return;
        }
        var normal = true;
        const sessionId = (ZEROES + Crypto.randomBytes(4).readUIntBE(0, 4) % 1000000).slice(-6);
        const client = Net.connect(targetPort, targetIp, () => {
            connections.set(sessionId, client);
            consoleLog(`Session ${sessionId} ${remoteIp} connected to ${targetIp}:${targetPort}`);
            server.getConnections((err, count) => {
                if (err) {
                    consoleLog(err.message);
                } else {
                    consoleLog(`Total connections = ${count}`);
                }
            });
        });
        client.setTimeout(IDLE_TIMEOUT, () => {
            consoleLog(`Session ${sessionId} timeout`);
            socket.end();
        });
        socket.on('data', (data) => {
            if (normal) client.write(data);
        });
        socket.once('end', () => {
            connections.delete(sessionId);
            consoleLog(`Session ${sessionId} closed by ${remoteIp}`);
            client.end();
            server.getConnections((err, count) => {
                if (err) {
                    consoleLog(err.message);
                } else {
                    consoleLog(`Remaining connections = ${count}`);
                }
            });
        });
        client.on('data', (data) => {
            if (normal) socket.write(data);
        });
        client.once('end', () => {
            connections.delete(sessionId);
            var bytesRead = NumberFormat.format(socket.bytesRead);
            var bytesWritten = NumberFormat.format(socket.bytesWritten)
            consoleLog(`Session ${sessionId} ${remoteIp} disconnected from ${targetIp}:${targetPort} rx ${bytesRead} tx ${bytesWritten} bytes`);
            socket.end();
            server.getConnections((err, count) => {
                if (err) {
                    consoleLog(err.message);
                } else {
                    consoleLog(`Remaining connections = ${count}`);
                }
            });
        });
        // Socket exceptions - most likely to be read timeout or connection reset
        socket.on('error', (err) => {
            if ('ECONNRESET' == err.code) {
                // normal case when user is using Windows
                consoleLog(`Session ${sessionId} closed by ${remoteIp}`);
            } else {
                consoleLog(`Session ${sessionId} exception (${remoteIp}) - ${err.code}`);
            }
            normal = false;
            client.end();
            socket.end();
        });
        client.on('error', (err) => {
            consoleLog(`Exception for port-${targetPort} - ${err}`);
            // Caution: the error message may depend on locale and language
            if (err.message.startsWith('connect ETIMEDOUT') && targetPort == restart) {
                // Let process manager to restart this app
                consoleLog(`Stopping application because port-${targetPort} does not respond`);
                process.exit(1);
            }
            normal = false;
            socket.end();
            client.end();
        });
    });
}

async function main() {
    var fileName = Path.resolve(__dirname, 'proxy-config.json');
    consoleLog(APP_NAME);
    consoleLog(`Loading config from ${fileName}`);
    if (Fs.existsSync(fileName)) {
        const fd = Fs.openSync(fileName);
        const text = Fs.readFileSync(fd, 'utf-8');
        Fs.closeSync(fd);
        const json = JSON.parse(text);
        const command = json['discovery']['command'];
        const tag = json['discovery']['tag'];
        const index = json['discovery']['index'];
        const authorized = json['authorized'];
        const restart = json['restart'];
        consoleLog(`Authorized users ${JSON.stringify(authorized)}`);
        const source_ports = json['source_ports'];
        const target_ports = json['target_ports'];
        if (command && tag && index > -1 && source_ports && target_ports) {
            if (source_ports.length != target_ports.length) {
                consoleLog('Invalid proxy-config.json');
            } else {
                // Obtain dynamic IP address - this assumes we are using multipass and the VM is called "main"
                const [valid, targetIp] = await getVmIpAddress(command, tag, index);
                if (valid) {
                    for (i in source_ports) {
                        forwardPort(source_ports[i], targetIp, target_ports[i], authorized, restart);
                    }
                } else {
                    consoleLog(targetIp);
                }
            }
        } else {
            consoleLog('Invalid proxy-config.json');
        }
    } else {
        consoleLog('Missing proxy-config.json');
    }
}

// start main application
main();
