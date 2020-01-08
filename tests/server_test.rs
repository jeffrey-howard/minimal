use minimal::Server;

#[test]
fn server_bad_address() {
    let mut server = Server::new();

    server.serve().expect_err("Should fail with a bad address.");
}
