# TCP tunnel over s3 cloud

For now this creates a tcp tunnel over amazon s3 cloud service.  
One usage could be if you don't have a direct access to the machine over ssh, so you can connect there using this tunnel.  
App uses two files for communication there and back.  
The next feature is to implement [mosh](https://mosh.org/) communication.  

For s3 client, app uses [aws_sdk_rust](https://github.com/lambdastackio/aws-sdk-rust).

## Usage:
You need a config file for the s3 client.
It is a simple yaml file:
``` yaml
---
access_key: YOURACCESSKEY
bucket_name: your_bucket_name
bucket_prefix: your_prefix
bucket_location: eu-west-1
secret_key: PUT-HERE-YOUR-SECRET-KEY
```

For the s3cmd tunnel version, the credentials are going to be used by the s3cmd configuration directly. Only needed are bucket_name and bucket_prefix.

The app's help prints this:

```
tunnel 0.1.2
Anicka Burova <anicka.burova@gmail.com>
Tunnel over various methods

USAGE:
    tunnel [OPTIONS] <mode>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --client-address <client-address>    Address where to connect on a new connection for the client mode [default: 127.0.0.1]
        --client-port <client-port>          Port where to connect on a new connection for the client mode [default: 22]
        --log-config <log-config>            Log configuration [default: log.yaml]
    -p, --server-port <server-port>          What port to listen on [default: 1234]
        --tunnel-api <tunnel-api>            What tunnel client to use [default: aws]  [values: aws, s3cmd]

ARGS:
    <mode>    What mode to run the tunnel in [values: server, client]
```

The default tunnel api is the aws library. This is probably the fastest way (I haven't benchmark it tho).   

To start the server, on your local machine from where you want to initiate ssh, run
```
tunnel server
```
Change the listening port to your favourite value.  

On the destination machine run
```
tunnel client
```
By default, this will create connection to localhost:22. This is standard ssh.

Now connect to your local machine ip using login name of the destination machine. Connect to the specified port. 
```
ssh destination_login@localhost -p 1234
```

This will create ssh connection to the destination machine.

## Limitations
* There is only one connection for now, after that connection is finished, both instances need to be restarted. (this will be fixed/improved soon).
* Communication will be very slow, because ssh sends every key press (might be improved by using mosh)
