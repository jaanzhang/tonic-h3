use std::{net::SocketAddr, time::Duration};

use tokio_util::sync::CancellationToken;
use tonic::transport::Uri;

#[tokio::test]
async fn h3_quinn_test() {
    h3_test(crate::run_test_quinn_hello_server).await;
}

#[tokio::test]
async fn h3_s2n_test() {
    h3_test(crate::run_test_s2n_server).await;
}

// takes in the fn to start the server and then send request to the server.
#[allow(clippy::type_complexity)]
async fn h3_test(
    run_server: fn(SocketAddr, CancellationToken) -> (tokio::task::JoinHandle<()>, SocketAddr),
) {
    crate::try_setup_tracing();

    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let token = CancellationToken::new();
    let (h_svr, listen_addr) = run_server(addr, token.clone());
    tracing::debug!("listenaddr : {}", listen_addr);

    // send client request
    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::debug!("connecting quic client.");

    let uri: Uri = format!("https://{listen_addr}").parse().unwrap();

    let client_endpoint = crate::make_test_quinn_client_endpoint();
    // quinn client test
    {
        // client drop is required to end connection. drive will end after connection end
        let cc = tonic_h3::quinn::H3QuinnConnector::new(
            uri.clone(),
            "localhost".to_string(),
            client_endpoint.clone(),
        );
        let channel = tonic_h3::H3Channel::new(cc, uri.clone());
        let mut client = crate::greeter_client::GreeterClient::new(channel);

        {
            let request = tonic::Request::new(crate::HelloRequest {
                name: "Tonic".into(),
            });
            let response = client.say_hello(request).await.unwrap();

            tracing::debug!("RESPONSE={:?}", response);
        }
        {
            let request = tonic::Request::new(crate::HelloRequest {
                name: "Tonic2".into(),
            });
            let response = client.say_hello(request).await.unwrap();

            tracing::debug!("RESPONSE={:?}", response);
        }
    }
    tracing::debug!("client wait idle");
    client_endpoint.wait_idle().await;

    // test s2n client
    let mut s2n_ep = crate::make_test_s2n_client_endpoint();
    {
        let cc = tonic_h3::s2n::client::H3S2nConnector::new(
            uri.clone(),
            "localhost".to_string(),
            s2n_ep.clone(),
        );
        let channel = tonic_h3::H3Channel::new(cc, uri.clone());
        let mut client = crate::greeter_client::GreeterClient::new(channel);
        {
            let request = tonic::Request::new(crate::HelloRequest {
                name: "Tonic-S2n".into(),
            });
            let response = client.say_hello(request).await.unwrap();
            tracing::debug!("RESPONSE={:?}", response);
        }
    }
    s2n_ep.wait_idle().await.unwrap();

    token.cancel();
    h_svr.await.unwrap();
}
