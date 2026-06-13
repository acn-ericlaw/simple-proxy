const process = require('process');
const Net = require('net');
const Crypto = require('crypto');
const NumberFormat = new Intl.NumberFormat();

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

const forwardPort = (sourceIp, sourcePort, targetIp, targetPort) => {
    const connections = new Map();
    // Setup TCP socket server
    const server = new Net.Server();
    server.listen(sourcePort, sourceIp, () => {
        consoleLog(`Forwarding ${sourceIp}:${sourcePort} to ${targetIp}:${targetPort}`);
    });
    server.on('error', (e) => {
        if ('EADDRINUSE' == e.code) {
            consoleLog(`Port ${sourceIp}:${sourcePort} is already used`);
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
            consoleLog(`Proxy ${sourceIp}:${sourcePort} to ${targetIp}:${targetPort} stopped`);
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
            consoleLog(`Session ${sessionId} ${remoteIp} disconnected from ${targetIp}:${targetPort} rx ${bytesRead} tx ${bytesWritten}`);
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
            normal = false;
            socket.end();
            client.end();
        });
    });
}

function main() {
    const args = process.argv.slice(2);
    if (args.length == 2) {
        var source = get_address(args[0]);
        var target = get_address(args[1]);
        if (source != null && target != null) {
            forwardPort(source[0], source[1], target[0], target[1]);
            return;
        }
    }
    console.log("Usage: node port-forward.js source_ip:port target_ip:port");
}

function get_address(ip) {
    const colon = ip.lastIndexOf(':');
    if (colon > 2) {
        var result = [];
        result.push(ip.substring(0, colon));
        const port = ip.substring(colon+1);
        if (!isNaN(port)) {
            result.push(parseInt(port));
            return result;
        }
    }
    return null;
}

main()
