use common::TestClient;

mod common;

#[tokio::test]
async fn can_fetch_index() {
    let test_server = common::TestServer::spawn().await;

    let client = TestClient::new(test_server.get_addr());

    let response = client.send_request("").await;

    let index_from_file = common::read_file_wo_lastline("tests/gopherhole/index");

    assert_eq!(index_from_file, response);

    test_server.shutdown();
}
