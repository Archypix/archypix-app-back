use crate::ftp_server::ftp_auth::PMAuthenticator;
use crate::ftp_server::ftp_backend::Vfs;


async fn start_ftp_server(){
    let server = libunftp::ServerBuilder::with_authenticator(
        Box::new(|| Vfs::new()),
        std::sync::Arc::new(PMAuthenticator{})
    )
    .greeting("Welcome to your Archypix FTP file server")
        .passive_ports(50000..65535)
        .build()
        .expect("Failed to create FTP server");

    server.listen("127.0.0.1:2121").await.expect("Failed to listen on port 2121 for FTP server.");
}


