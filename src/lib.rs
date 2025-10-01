//! An async tls stream library based on [rustls] and [futures_io]. Both for server/client.
//!
//! 一个基于 [rustls] 和 [futures_io] 的异步 TLS 流库，同时支持服务端和客户端。
//!
//! # Examples
//!
//! **Server**
//! ```ignore
//! let listener = async_net::TcpListener::bind((Ipv4Addr::LOCALHOST, 4443)).await.unwrap();
//! let (stream, remote_addr) = listener.accept().await.unwrap();
//!
//! // Recv Client Hello / 接收客户端问候消息
//! let accept = TlsAccepted::accept(stream).await.unwrap();
//!
//! let server_config = Arc::new(server_config);
//! let mut stream = accept.into_stream(server_config.clone()).unwrap();
//! // handshake completed / 握手完成
//! stream.flush().await.unwrap();
//! ```
//!
//! **Client**
//!
//! ```ignore
//! let server_name = "test.com".try_into().unwrap();
//! let client_config = Arc::new(client_config);
//! let connector = TlsConnector::new(client_config.clone(), server_name).unwrap();
//!
//! let stream = async_net::TcpStream::connect((Ipv4Addr::LOCALHOST, 4443)).await.unwrap();
//!
//! let mut stream = connector.connect(stream);
//! // handshake completed / 握手完成
//! stream.flush().await.unwrap();
//! ```
//! or [examples](https://github.com/hs-CN/async-rustls-stream/blob/master/examples).

use std::{
  future::Future,
  io::{self, Read, Write},
  ops::{Deref, DerefMut},
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
use rustls::{
  ClientConfig, ClientConnection, ConnectionCommon, ServerConfig, ServerConnection, SideData,
  Stream, pki_types,
  server::{Accepted, Acceptor, ClientHello},
};

/// A wrapper that implements synchronous `Read` and `Write` for an asynchronous stream `T`.
/// This is a bridge between the synchronous I/O required by `rustls` and the asynchronous I/O of the underlying stream.
///
/// 一个为异步流 `T` 实现同步 `Read` 和 `Write` 的包装器。
/// 这是在 `rustls` 所需的同步 I/O 与底层流的异步 I/O 之间架起的一座桥梁。
struct InnerStream<'a, 'b, T> {
  cx: &'a mut Context<'b>,
  stream: &'a mut T,
}

impl<'a, 'b, T: AsyncRead + Unpin> Read for InnerStream<'a, 'b, T> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    match Pin::new(&mut self.stream).poll_read(self.cx, buf) {
      Poll::Ready(res) => res,
      Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
    }
  }

  fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
    match Pin::new(&mut self.stream).poll_read_vectored(self.cx, bufs) {
      Poll::Ready(res) => res,
      Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
    }
  }
}

impl<'a, 'b, T: AsyncWrite + Unpin> Write for InnerStream<'a, 'b, T> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    match Pin::new(&mut self.stream).poll_write(self.cx, buf) {
      Poll::Ready(res) => res,
      Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
    }
  }

  fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
    match Pin::new(&mut self.stream).poll_write_vectored(self.cx, bufs) {
      Poll::Ready(res) => res,
      Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
    }
  }

  fn flush(&mut self) -> io::Result<()> {
    match Pin::new(&mut self.stream).poll_flush(self.cx) {
      Poll::Ready(res) => res,
      Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
    }
  }
}

/// A TLS stream that implements [AsyncRead] and [AsyncWrite].
///
/// 一个实现了 [AsyncRead] 和 [AsyncWrite] 的 TLS 流。
pub struct TlsStream<C, T> {
  connection: C,
  stream: T,
}

impl<C, T> TlsStream<C, T> {
  /// Get immutable references to the underlying connection and stream.
  ///
  /// 获取底层连接和流的不可变引用。
  pub fn get_ref(&self) -> (&C, &T) {
    (&self.connection, &self.stream)
  }

  /// Get mutable references to the underlying connection and stream.
  ///
  /// 获取底层连接和流的可变引用。
  pub fn get_mut(&mut self) -> (&mut C, &mut T) {
    (&mut self.connection, &mut self.stream)
  }
}

impl<C, T, S> AsyncRead for TlsStream<C, T>
where
  C: DerefMut + Deref<Target = ConnectionCommon<S>> + Unpin,
  T: AsyncRead + AsyncWrite + Unpin,
  S: SideData,
{
  fn poll_read(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
    buf: &mut [u8],
  ) -> std::task::Poll<std::io::Result<usize>> {
    let (connection, stream) = (*self).get_mut();
    let mut stream = Stream {
      conn: connection,
      sock: &mut InnerStream { cx, stream },
    };
    match stream.read(buf) {
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      res => Poll::Ready(res),
    }
  }

  fn poll_read_vectored(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
    bufs: &mut [std::io::IoSliceMut<'_>],
  ) -> std::task::Poll<std::io::Result<usize>> {
    let (connection, stream) = (*self).get_mut();
    let mut stream = Stream {
      conn: connection,
      sock: &mut InnerStream { cx, stream },
    };
    match stream.read_vectored(bufs) {
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      res => Poll::Ready(res),
    }
  }
}

impl<C, T, S> AsyncWrite for TlsStream<C, T>
where
  C: DerefMut + Deref<Target = ConnectionCommon<S>> + Unpin,
  T: AsyncRead + AsyncWrite + Unpin,
  S: SideData,
{
  fn poll_write(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
    buf: &[u8],
  ) -> std::task::Poll<std::io::Result<usize>> {
    let (connection, stream) = (*self).get_mut();
    let mut stream = Stream {
      conn: connection,
      sock: &mut InnerStream { cx, stream },
    };
    match stream.write(buf) {
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      res => Poll::Ready(res),
    }
  }

  fn poll_write_vectored(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
    bufs: &[std::io::IoSlice<'_>],
  ) -> std::task::Poll<std::io::Result<usize>> {
    let (connection, stream) = (*self).get_mut();
    let mut stream = Stream {
      conn: connection,
      sock: &mut InnerStream { cx, stream },
    };
    match stream.write_vectored(bufs) {
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      res => Poll::Ready(res),
    }
  }

  fn poll_flush(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<std::io::Result<()>> {
    let (connection, stream) = (*self).get_mut();
    let mut stream = Stream {
      conn: connection,
      sock: &mut InnerStream { cx, stream },
    };
    match stream.flush() {
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      res => Poll::Ready(res),
    }
  }

  fn poll_close(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<std::io::Result<()>> {
    self.poll_flush(cx)
  }
}

/// A TLS client connector for initiating a TLS handshake.
///
/// 用于发起 TLS 握手的 TLS 客户端连接器。
///
/// Use [TlsConnector::connect()] to get a [TlsStream] for the client.
/// Then use [TlsStream::flush()] to complete the handshake.
///
/// 使用 [TlsConnector::connect()] 为客户端获取 [TlsStream]。
/// 然后使用 [TlsStream::flush()] 完成握手。
pub struct TlsConnector(ClientConnection);

impl TlsConnector {
  /// Creates a new TLS connector.
  ///
  /// 创建一个新的 TLS 连接器。
  pub fn new(
    config: Arc<ClientConfig>,
    server_name: pki_types::ServerName<'static>,
  ) -> Result<Self, rustls::Error> {
    let connection = ClientConnection::new(config, server_name)?;
    Ok(Self(connection))
  }

  /// Connects to a server and returns a `TlsStream`.
  /// The `stream` should generally implement [AsyncRead] and [AsyncWrite].
  ///
  /// 连接到服务器并返回一个 `TlsStream`。
  /// `stream` 通常应实现 [AsyncRead] 和 [AsyncWrite]。
  pub fn connect<T>(self, stream: T) -> TlsStream<ClientConnection, T> {
    TlsStream {
      connection: self.0,
      stream,
    }
  }
}

/// Represents a server-side TLS session after the `Client Hello` has been received.
///
/// 表示在收到 `Client Hello` 后的服务器端 TLS 会话。
///
/// Use [`TlsAccepted::accept()`] to receive the `Client Hello`.
/// Then use [TlsAccepted::into_stream()] to get a [TlsStream].
/// Then use [TlsStream::flush()] to complete the handshake.
///
/// 使用 [`TlsAccepted::accept()`] 接收 `Client Hello`。
/// 然后使用 [TlsAccepted::into_stream()] 获取 [TlsStream]。
/// 然后使用 [TlsStream::flush()] 完成握手。
pub struct TlsAccepted<T> {
  accepted: Accepted,
  stream: T,
}

impl<T> TlsAccepted<T> {
  /// Gets the [`ClientHello`] message received from the client.
  ///
  /// 获取从客户端收到的 [`ClientHello`] 消息。
  pub fn client_hello(&self) -> ClientHello<'_> {
    self.accepted.client_hello()
  }

  /// Converts this into a [`TlsStream`] with the given [`ServerConfig`].
  ///
  /// 使用给定的 [`ServerConfig`] 将其转换为 [`TlsStream`]。
  pub fn into_stream(
    self,
    config: Arc<ServerConfig>,
  ) -> Result<TlsStream<ServerConnection, T>, rustls::Error> {
    let connection = self.accepted.into_connection(config).map_err(|(e, _)| e)?;
    Ok(TlsStream {
      connection,
      stream: self.stream,
    })
  }
}

impl<T> TlsAccepted<T>
where
  T: AsyncRead + Unpin,
{
  /// Asynchronously accepts a new connection, receiving the `Client Hello`.
  /// The `stream` should generally implement [AsyncRead] and [AsyncWrite].
  ///
  /// 异步接受一个新连接，并接收 `Client Hello`。
  /// `stream` 通常应实现 [AsyncRead] 和 [AsyncWrite]。
  pub async fn accept(mut stream: T) -> io::Result<TlsAccepted<T>> {
    let accepted = AcceptFuture {
      acceptor: Acceptor::default(),
      stream: &mut stream,
    }
    .await?;
    Ok(TlsAccepted { accepted, stream })
  }
}

/// A future that resolves when the `Client Hello` has been received.
///
/// 一个在收到 `Client Hello` 后完成的 future。
struct AcceptFuture<'a, T> {
  acceptor: Acceptor,
  stream: &'a mut T,
}

impl<'a, T> AcceptFuture<'a, T> {
  fn get_mut(&mut self) -> (&mut Acceptor, &mut T) {
    (&mut self.acceptor, self.stream)
  }
}

impl<'a, T: AsyncRead + Unpin> Future for AcceptFuture<'a, T> {
  type Output = io::Result<Accepted>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let (acceptor, stream) = (*self).get_mut();
    // Attempt to read TLS data from the underlying stream.
    // 尝试从底层流中读取 TLS 数据。
    match acceptor.read_tls(&mut InnerStream { cx, stream }) {
      Ok(_) => {
        // If data is read, try to accept a new session.
        // 如果读取到数据，则尝试接受一个新的会话。
        match self.acceptor.accept() {
          Ok(None) => Poll::Pending, // Not enough data yet / 数据还不够
          Ok(Some(accepted)) => Poll::Ready(Ok(accepted)), // Session accepted / 会话已接受
          Err(err) => Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidData, err.0))),
        }
      }
      // If the read would block, return Pending.
      // 如果读取将阻塞，则返回 Pending。
      Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
      // Propagate other errors.
      // 传播其他错误。
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

#[cfg(test)]
mod test;
