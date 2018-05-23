#[macro_use]
extern crate clap;
#[macro_use]
extern crate tokio_core;

extern crate futures;

use clap::{App, Arg};
use futures::{Future, Poll};
use std::str;
use std::net::SocketAddr;
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

fn main() {
    let app = App::new("multicaster")
        .about("receive and forward multicast datagrams")
        .version(crate_version!())
        .author("Jason Longshore <hello@jasonlongshore.com>")
        .arg(
            Arg::with_name("source-host")
                .required(true)
                .takes_value(true)
                .long("source-host"))
        .arg(
            Arg::with_name("source-port")
                .required(true)
                .takes_value(true)
                .long("source-port")
                .validator(arg_validate_parse::<i32>))
        .arg(
            Arg::with_name("destination-host")
                .required(true)
                .takes_value(true)
                .long("destination-host"))
        .arg(
            Arg::with_name("destination-port")
                .required(true)
                .takes_value(true)
                .long("destination-port")
                .validator(arg_validate_parse::<i32>));

    let args = app.get_matches();

    // unwrap in the following is safe because clap has validated for us
    let source_host = args.value_of("source-host").unwrap();
    let source_port: i32 = args.value_of("source-port").unwrap().parse().unwrap();
    let destination_host = args.value_of("destination-host").unwrap();
    let destination_port: i32 = args.value_of("destination-port").unwrap().parse().unwrap();

    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let socket = UdpSocket::bind(&format!("0.0.0.0:{}", source_port).parse().unwrap(), &handle).unwrap();

    socket.join_multicast_v4(&source_host.parse().unwrap(), &"0.0.0.0".parse().unwrap())
          .unwrap();

    let server = Server {
        buf: vec![0; 4096],
        data: None,
        forward: format!("{}:{}", destination_host, destination_port).parse().unwrap(),
        socket: socket
    };


    if let Err(err) = l.run(server) {
        panic!("crashed: {:?}", err);
    }
}

fn arg_validate_parse<T: str::FromStr>(arg: String) -> Result<(), String> {
    arg
        .parse::<T>()
        .map(|_| ())
        .map_err(|_| format!("unable to parse: {}", arg))
}

struct Server {
    pub buf: Vec<u8>,
    pub data: Option<(usize, SocketAddr)>,
    pub forward: SocketAddr,
    pub socket: UdpSocket
}

impl Future for Server {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), std::io::Error> {
        loop {
            if let Some((size, peer)) = self.data {
                let data = &self.buf[..size];

                println!("data len: {:?}", data.len());

                let amt = try_nb!(self.socket.send_to(&data, &self.forward));

                if amt != data.len() {
                    // error!("failed to write entire message: {}/{} bytes sent to {}", amt, data.len(), peer); // @TODO logging
                }
                // we'll now need to forward it

                self.data = None;
            }

            self.data = Some(try_nb!(self.socket.recv_from(&mut self.buf)));
        }
    }
}
