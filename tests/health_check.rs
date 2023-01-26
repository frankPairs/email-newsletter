use std::net::TcpListener;

fn spawn_app() -> String {
    // We are using port 0 as way to define a different port per each test. Port 0 is a special case that operating systems
    // take into account: when port is 0, the OS will search for the first available port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = email_newsletter::run(listener).expect("Faild to bind address");

    tokio::spawn(server);

    format!("127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let client = reqwest::Client::new();
    let url = format!("http://{}/health_check", address);
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}
