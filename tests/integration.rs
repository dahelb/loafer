use common::TestClient;

mod common;

#[test]
fn can_fetch_index() {
    let test_server = common::TestServer::spawn();

    let client = TestClient::new(test_server.get_addr());

    let response = client.send_request("");

    let index_from_file = std::fs::read_to_string("tests/gopherhole/index").unwrap();

    assert_eq!(index_from_file, response);

    test_server.shutdown();
}
