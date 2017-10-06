/**
 * File: src/s3tunnel_cmd.rs
 * Author: Anicka Burova <anicka.burova@gmail.com>
 * Date: 11.09.2017
 * Last Modified Date: 06.10.2017
 * Last Modified By: Anicka Burova <anicka.burova@gmail.com>
 */

use std::io::{self};
use serde_yaml;
use config::S3Config;

use std::sync::mpsc::{channel};
use tunnel::*;
use messages::*;

macro_rules! shell {
    (run => $cmd: expr, [ $($arg: expr ), * ] ) => ({
        let args = vec![$($arg,)*];
        info!("{} {:?}", $cmd, args);
        use std::process::*;
        Command::new($cmd)
                .args(&[$($arg,)*])
                .status()
    });
    (pipe => $file: expr, $cmd: expr, [ $($arg: expr ), * ] ) => ({
        use std::process::*;
        use std::fs::File;
        use std::io::Write;
        let cmd = Command::new($cmd)
                .args(&[$($arg,)*])
                .stdout(Stdio::piped())
                .spawn()?;
        let output = cmd.wait_with_output()?.stdout;
        let mut file = File::create($file)?;
        file.write_all(&output)?;
    });
}


macro_rules! delete_files {
    ( $bucket_name: ident, $bucket_prefix: ident, $files: expr) => {
        for file in $files.iter() {
            let file_name = format!("s3://{}/{}/{}", $bucket_name, $bucket_prefix, file);
            let _ = shell!(run => "s3cmd", ["del", &file_name]).expect("Delete failed");
        }
    }
}

pub fn create_clients(is_server: bool, cfg: S3Config, writer_name: &str, reader_name: &str) -> io::Result<TunnelPipes> {
    let bucket_name = cfg.bucket_name;
    let bucket_prefix = cfg.bucket_prefix;

    let (writer_sender, writer_receiver) = channel::<WriteCommand>();
    let (reader_sender, reader_receiver) = channel::<ReadCommand>();

    use std::thread;

    info!("Reading data from '{}'", reader_name);
    info!("Writing data to '{}'", writer_name);
    // create thread own variables to avoid lifetime errors
    let reader_name: String = reader_name.into();
    let writer_name: String = writer_name.into();

    thread::spawn(move|| {
        if is_server {
            delete_files!( bucket_name, bucket_prefix, vec![&reader_name, &writer_name]);
        }
        let reader = format!("s3://{}/{}/{}", bucket_name, bucket_prefix, reader_name);
        let bucket_place = format!("s3://{}/{}/", bucket_name, bucket_prefix);
        loop {
            for cmd in writer_receiver.try_iter() {
                match cmd {
                    WriteCommand::Write(msg) => {
                        use std::fs::File;
                        use std::io::Write;
                        info!("Sending the data");
                        let _ = File::create(&writer_name)
                            .and_then(|mut file| {
                                info!("Writing the file");
                                file.write_all(&msg)
                            })
                            .and_then(|_| {
                                info!("Uploading the file");
                                shell!(run => "s3cmd", ["put", &writer_name, &bucket_place])
                            })
                            .and_then(|_| {
                                info!("Message writes, size = {}", msg.len());
                                Ok(())
                            })
                            .unwrap();
                    }
                    WriteCommand::Delete => {
                        delete_files!(bucket_name, bucket_name, vec![&writer_name]);
                    }
                }
            }

            let _ = shell!(run => "s3cmd", ["get", &reader, "--force"])
                .and_then(|status| {
                    match status.code() {
                        Some(0) => {
                            use std::fs::File;
                            use std::io::Read;
                            File::open(&reader_name)
                                .and_then(|mut file| {
                                    let mut contents = Vec::new();
                                    file.read_to_end(&mut contents)
                                        .and_then(|_| {
                                            use std::str;
                                            let text = str::from_utf8(&contents).unwrap();
                                            let stream: Vec<Message> = serde_yaml::from_str(text).unwrap();

                                            info!("Sending data to channel");
                                            let _ = reader_sender.send(ReadCommand::Read(stream)).unwrap();
                                            //sleep(Duration::from_millis(800));
                                            Ok(())
                                        })
                                })
                        }
                        Some(12) => {
                            // TODO: remove NoFile
                            let _ = reader_sender.send(ReadCommand::NoFile).unwrap();
                            //sleep(Duration::from_millis(2000));
                            Ok(())
                        }
                        err => {
                            error!("Failed to get the {} from s3: {:?}", reader_name, err);
                            Ok(())
                        }
                    }
                })
                .unwrap();

            //match client.get_object(&reader, None) {
                //Ok(output) => {
                    //use std::str;
                    //let text = str::from_utf8(&output.body).unwrap();
                    //let stream: Vec<Message> = serde_yaml::from_str(text).unwrap();

                    //let _ = reader_sender.send(ReadCommand::Read(stream)).unwrap();
                    //sleep(Duration::from_millis(800));
                //}
                //Err(_) => {
                    //let _ = reader_sender.send(ReadCommand::NoFile).unwrap();
                    //// if the file is missing, just wait until it is created
                    ////info!("{} not found", reader_name);
                    //sleep(Duration::from_millis(2000));
                //}
            //}
        }
    });


    Ok(TunnelPipes{
        writer: writer_sender,
        reader: reader_receiver,
    })
}
