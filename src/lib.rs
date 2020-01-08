use actix::System;
use actix_web::dev;
use actix_web::{App, HttpServer};
use std::fmt::Debug;
use std::fmt::{Display, Formatter};
use std::option::Option;
use std::sync::mpsc;
use std::sync::mpsc::RecvError;
use std::sync::mpsc::Sender;
use std::thread::{spawn, JoinHandle};

#[derive(Debug)]
pub enum ErrorKind {
    AddrInUse(std::io::Error),
    AddrNotAvailable(std::io::Error),
    ServerStartupFailed(RecvError),
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(kind: ErrorKind, source: Option<Box<dyn std::error::Error + Send + Sync>>) -> Error {
        Error { kind, source }
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        write!(
            fmt,
            "ErrorKind: {:#?}, source: {:#?}",
            self.kind, self.source
        )
    }
}

impl std::error::Error for Error {}

pub struct Server {
    handle: Option<JoinHandle<()>>,
    server: Option<dev::Server>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            handle: None,
            server: None,
        }
    }

    pub fn serve(&mut self) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel();

        self.handle = Some(spawn(move || Server::start(tx)));

        rx.recv()
            .map_err(|e| Error::new(ErrorKind::ServerStartupFailed(e), None))
            .and_then(|m| match m {
                Ok(server) => {
                    self.server = Some(server);
                    Ok(())
                }
                Err(e) => match self.handle.take() {
                    Some(h) => {
                        let _ = h.join();
                        Err(e)
                    }
                    None => Err(e),
                },
            })
    }

    fn start(tx: Sender<Result<dev::Server, Error>>) {
        let sys = System::new("minimal");

        let _ = HttpServer::new(move || App::new())
            .bind("1.2.3.4:5")
            .and_then(|s| {
                let server = s.system_exit().start();
                let _ = tx.send(Ok(server));
                let _ = sys.run();
                Ok(())
            })
            .map_err(|e| {
                let k = match e.kind() {
                    std::io::ErrorKind::AddrInUse => ErrorKind::AddrInUse(e),
                    _ => ErrorKind::AddrNotAvailable(e),
                };
                let _ = tx.send(Err(Error::new(k, None)));
            });
    }
}
