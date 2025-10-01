use std::{io::Cursor, net::Ipv4Addr, sync::Arc, time::Duration};

use async_executor::Executor;
use futures_lite::prelude::*;
use rustls::ServerConfig;

use crate::{TlsAccepted, TlsConnector};

static EX: Executor = Executor::new();

#[test]
fn server_client_test() {
  // --- Certificate and Key Loading ---
  // --- 证书与密钥加载 ---
  let ca_cert: Vec<_> = rustls_pemfile::certs(&mut Cursor::new(include_bytes!("../cert/ca.crt")))
    .unwrap()
    .into_iter()
    .map(|x| rustls::Certificate(x))
    .collect();
  let ca_key: Vec<_> =
    rustls_pemfile::rsa_private_keys(&mut Cursor::new(include_bytes!("../cert/ca.key")))
      .unwrap()
      .into_iter()
      .map(|x| rustls::PrivateKey(x))
      .collect();

  // --- Server Setup ---
  // --- 服务器设置 ---
  let server_config = ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(ca_cert.clone(), ca_key[0].clone())
    .unwrap();
  let server_config = Arc::new(server_config);
  EX.spawn(async move {
    let listener = async_net::TcpListener::bind((Ipv4Addr::LOCALHOST, 4443))
      .await
      .unwrap();
    loop {
      let (stream, _remote_addr) = listener.accept().await.unwrap();
      let server_config = server_config.clone();
      // Spawn a task for each incoming connection.
      // 为每个传入的连接生成一个任务。
      EX.spawn(async move {
        // Accept the TLS connection. / 接受 TLS 连接。
        let accept = TlsAccepted::accept(stream).await.unwrap();
        let mut stream = accept.into_stream(server_config.clone()).unwrap();
        stream.flush().await.unwrap(); // Handshake complete / 握手完成

        // Read data from client. / 从客户端读取数据。
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        let str = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(str, "server and client test");

        // Echo data back to client. / 将数据回显给客户端。
        stream.write_all(&buf[..n]).await.unwrap();
        stream.flush().await.unwrap();
      })
      .detach();
    }
  })
  .detach();

  // --- Client Setup ---
  // --- 客户端设置 ---
  let mut root_store = rustls::RootCertStore::empty();
  root_store.add(&ca_cert[0]).unwrap();
  let config = Arc::new(
    rustls::ClientConfig::builder()
      .with_safe_defaults()
      .with_root_certificates(root_store)
      .with_no_client_auth(),
  );
  let server_name = "test.com".try_into().unwrap();

  // --- Client Task ---
  // --- 客户端任务 ---
  let client_task = EX.spawn(async move {
    // Create a connector and connect. / 创建连接器并连接。
    let connector = TlsConnector::new(config.clone(), server_name).unwrap();
    let stream = async_net::TcpStream::connect((Ipv4Addr::LOCALHOST, 4443))
      .await
      .unwrap();
    let mut stream = connector.connect(stream);
    stream.flush().await.unwrap(); // Handshake complete / 握手完成
    async_io::Timer::after(Duration::from_millis(1)).await;

    // Write data to server. / 向服务器写入数据。
    let line = "server and client test";
    stream.write_all(line.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();

    // Read echoed data from server and verify. / 从服务器读取回显数据并验证。
    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let back = String::from_utf8_lossy(&buf[..n]);
    assert_eq!(line, back);
  });

  // Run the client task to completion.
  // 运行客户端任务直至完成。
  async_io::block_on(EX.run(client_task));
}
