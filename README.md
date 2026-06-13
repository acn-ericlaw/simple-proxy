# Simple TCP socket proxy

This proxy was originally designed to allow a user to ssh into a Ubuntu multipass VM from the host machine or the Internet.

I am amazed about the simplicity and performance of the proxy, thus polishing it to become a generic socket proxy that you can use to do network address translation (NAT) from a host port to a guest VM target IP and port.

## proxy-config.json

The IP discovery command, source and target ports are configurable using a JSON file like this. You may adjust the configuration file to fit your use case.

```json
{
  "discovery": {
    "command": "multipass exec main ifconfig eth0",
    "tag": "inet",
    "index": 1
  },
  "source_port": [22],
  "target_port": [22],
  "authorized": ["192.168.1.*"]
}
```

If you want the proxy to detect hyper-v VM in Windows directly, you can replace proxy-config.json with proxy-config-hyperv.json.

There are two sample proxy config JSON files:

1. proxy-config-multipass.json
2. proxy-config-hyperv.json

You can adjust the config file to fit your use case.

## SSH security

If you expose your guest VM to the Internet, make sure you enforce certificate authentication. i.e. disable SSH login using password.

Please create a "non-root" user and save the public key of the user in the "authorized_keys" file under the ".ssh" folder.
For security reason, please use certificate with RSA key length of 4,096 bits.

## Running this utility as a service

We recommend that you use Node's PM2 process manager or docker-compose to deploy the utility as a service.

It is as simple as:

```
pm2 start simple-proxy.js
```
This assumes you start the utility from the project directory. If not, please add path to the filename.

## Library dependencies

Please install the moment time utility before you start the utility.

```
cd simple-proxy
npm install
```

## Port-Forward utility

A convenient port-forward utility is derived from the simple-proxy app.
This port-forward utility is particularly useful when using VMs in your laptop.
It allows you to reach the VM's applications (docker/kubernetes, etc.) from the host OS.

The port-forward utility is a command line tool. You can run it like this:

```
node port-forward.js source_ip:port target_ip:port
```
To listen to all IP addresses of the host, you may use 0.0.0.0 as the source_ip


## Idle disconnect timer

The system has an idle disconnect timeout value of 30 minutes, you may adjust it according to your use cases.
