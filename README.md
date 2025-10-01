[English](#english) | [中文](#中文)

<a name="english"></a>

# rustls_async

An async TLS stream wrapper library based on [rustls](https://crates.io/crates/rustls) and [futures-io](https://crates.io/crates/futures-io), providing a unified interface for both server and client applications.

## Table of Contents

- [Project Overview](#project-overview)
- [Features](#features)
- [Technology Stack](#technology-stack)
- [Design Philosophy](#design-philosophy)
- [Usage Examples](#usage-examples)
- [File Structure](#file-structure)
- [A Little Story](#a-little-story)

## Project Overview

This library provides a straightforward abstraction layer over the powerful `rustls` TLS library, adapting it for the asynchronous ecosystem in Rust. It utilizes the `futures-io` traits (`AsyncRead`, `AsyncWrite`), making it compatible with various async runtimes like `async-std`, `smol`, and others that support these traits. The goal is to offer a simple and secure way to establish TLS-encrypted connections without being tied to a specific runtime.

## Features

- **Async Operation**: Fully asynchronous TLS stream handling.
- **Runtime Agnostic**: Compatible with any async runtime that supports `futures-io`.
- **Server & Client**: Provides APIs for both TLS server and client roles.
- **Security**: Built on top of `rustls`, a modern, secure, and audited TLS library.

## Technology Stack

- **[rustls](https://crates.io/crates/rustls)**: Used for the underlying TLS session management and cryptography. It provides the security guarantees.
- **[futures-io](https://crates.io/crates/futures-io)**: Used for the asynchronous I/O traits, which allows the library to remain independent of any specific async runtime.

## Design Philosophy

The library's design is centered around two primary structs that simplify the TLS handshake process: `TlsConnector` for clients and `TlsAccepted` for servers.

### Client Workflow

1.  **`TlsConnector::new(config, server_name)`**: A connector is created with a client configuration and the server's hostname.
2.  **`connector.connect(stream)`**: The connector takes an existing async stream (e.g., a TCP stream) and returns a `TlsStream`.
3.  **`stream.flush()`**: The final step of the handshake is completed by flushing the stream, after which encrypted data can be sent and received.

### Server Workflow

1.  **`TlsAccepted::accept(stream)`**: The server accepts a raw incoming stream and performs a preliminary read to parse the "Client Hello" message without blocking.
2.  **`accept.into_stream(config)`**: The `TlsAccepted` object, which now contains the client's initial request, is combined with a server configuration to create the `TlsStream`.
3.  **`stream.flush()`**: Similar to the client, flushing the stream completes the server-side handshake.

Internally, a struct named `InnerStream` acts as a bridge. During each `poll` call, it wraps the async stream (`T`) and implements the standard blocking `std::io::Read` and `std::io::Write` traits. This allows the synchronous `rustls` core to process data from the async stream within the non-blocking context of a future.

## Usage Examples

### Server

```rust
use std::net::Ipv4Addr;
use std::sync::Arc;
use futures_lite::io::AsyncWriteExt;
use rustls_async::TlsAccepted;

// Assume `listener` is an async TCP listener and `server_config` is a valid rustls::ServerConfig
let (stream, _remote_addr) = listener.accept().await.unwrap();

// Receive Client Hello
let accept = TlsAccepted::accept(stream).await.unwrap();

// Create TlsStream and complete handshake
let server_config = Arc::new(server_config);
let mut stream = accept.into_stream(server_config.clone()).unwrap();
stream.flush().await.unwrap();
```

### Client

```rust
use std::net::Ipv4Addr;
use std::sync::Arc;
use futures_lite::io::AsyncWriteExt;
use rustls_async::TlsConnector;
use rustls::pki_types::ServerName;

// Assume `client_config` is a valid rustls::ClientConfig
let server_name: ServerName<'static> = "test.com".try_into().unwrap();
let client_config = Arc::new(client_config);
let connector = TlsConnector::new(client_config.clone(), server_name).unwrap();

// Connect to server
let stream = async_net::TcpStream::connect((Ipv4Addr::LOCALHOST, 4443)).await.unwrap();

// Create TlsStream and complete handshake
let mut stream = connector.connect(stream);
stream.flush().await.unwrap();
```

For a complete, runnable example, see the code in the `examples/` directory.

## File Structure

```
/
├── Cargo.toml       # Project metadata and dependencies
├── src/
│   ├── lib.rs       # Core library code
│   └── test.rs      # Integration tests
├── examples/
│   └── server_client.rs # Runnable usage example
└── cert/            # Scripts and configs for generating test certificates
```

## A Little Story

The `rustls` library, which this project depends on, was created partly in response to a desire for a more modern and memory-safe alternative to OpenSSL. The security landscape of the internet was dramatically altered in 2014 by the "Heartbleed" bug in OpenSSL. This vulnerability allowed attackers to read the memory of servers, exposing sensitive data like private keys, passwords, and personal user information. It was a stark reminder of the dangers of memory safety issues (like buffer over-reads) in critical security software.

`rustls` leverages the safety guarantees of the Rust language to eliminate entire classes of bugs, including the one that caused Heartbleed. By building on `rustls`, `rustls_async` inherits these security benefits, contributing to a more secure ecosystem for asynchronous applications in Rust.

---

<a name="中文"></a>

# rustls_async

一个基于 [rustls](https://crates.io/crates/rustls) 和 [futures-io](https://crates.io/crates/futures-io) 的异步 TLS 流封装库，为服务端和客户端应用提供统一的接口。

## 目录

- [项目概述](#项目概述-chinese)
- [功能特性](#功能特性-chinese)
- [技术堆栈](#技术堆栈-chinese)
- [设计思路](#设计思路-chinese)
- [使用示例](#使用示例-chinese)
- [文件结构](#文件结构-chinese)
- [相关故事](#相关故事-chinese)

## <a name="项目概述-chinese"></a>项目概述

本库在强大的 `rustls` TLS 库之上提供了一个直接的抽象层，使其适配 Rust 的异步生态系统。它利用 `futures-io` 的 trait（`AsyncRead`, `AsyncWrite`），使其能够兼容多种异步运行时，如 `async-std`、`smol` 等。项目的目标是提供一种简单且安全的方式来建立 TLS 加密连接，同时不与任何特定的运行时绑定。

## <a name="功能特性-chinese"></a>功能特性

- **异步操作**: 完全异步的 TLS 流处理。
- **运行时无关**: 兼容任何支持 `futures-io` 的异步运行时。
- **服务端与客户端**: 同时提供 TLS 服务端和客户端的 API。
- **安全性**: 构建于 `rustls` 之上，这是一个现代、安全且经过审计的 TLS 库。

## <a name="技术堆栈-chinese"></a>技术堆栈

- **[rustls](https://crates.io/crates/rustls)**: 用于底层的 TLS 会话管理和加密，提供安全保障。
- **[futures-io](https://crates.io/crates/futures-io)**: 用于异步 I/O trait，使该库能够独立于任何特定的异步运行时。

## <a name="设计思路-chinese"></a>设计思路

该库的设计围绕两个主要结构体展开，以简化 TLS 握手过程：用于客户端的 `TlsConnector` 和用于服务端的 `TlsAccepted`。

### 客户端工作流

1.  **`TlsConnector::new(config, server_name)`**: 使用客户端配置和服务器主机名创建一个连接器。
2.  **`connector.connect(stream)`**: 连接器接收一个已有的异步流（例如 TCP 流），并返回一个 `TlsStream`。
3.  **`stream.flush()`**: 通过刷新流来完成握手的最后一步，之后便可以收发加密数据。

### 服务端工作流

1.  **`TlsAccepted::accept(stream)`**: 服务器接收一个原始的入站流，并执行初步读取以无阻塞地解析 "Client Hello" 消息。
2.  **`accept.into_stream(config)`**: 将包含客户端初始请求的 `TlsAccepted` 对象与服务器配置结合，以创建 `TlsStream`。
3.  **`stream.flush()`**: 与客户端类似，刷新流以完成服务端的握手。

在内部，一个名为 `InnerStream` 的结构体充当桥梁。在每次 `poll` 调用期间，它会包装异步流（`T`）并实现标准的阻塞 `std::io::Read` 和 `std::io::Write` trait。这使得同步的 `rustls` 核心能够在一个 future 的非阻塞上下文中处理来自异步流的数据。

## <a name="使用示例-chinese"></a>使用示例

### 服务端

```rust
use std::net::Ipv4Addr;
use std::sync::Arc;
use futures_lite::io::AsyncWriteExt;
use rustls_async::TlsAccepted;

// 假设 `listener` 是一个异步 TCP 监听器，`server_config` 是一个有效的 rustls::ServerConfig
let (stream, _remote_addr) = listener.accept().await.unwrap();

// 接收 Client Hello
let accept = TlsAccepted::accept(stream).await.unwrap();

// 创建 TlsStream 并完成握手
let server_config = Arc::new(server_config);
let mut stream = accept.into_stream(server_config.clone()).unwrap();
stream.flush().await.unwrap();
```

### 客户端

```rust
use std::net::Ipv4Addr;
use std::sync::Arc;
use futures_lite::io::AsyncWriteExt;
use rustls_async::TlsConnector;
use rustls::pki_types::ServerName;

// 假设 `client_config` 是一个有效的 rustls::ClientConfig
let server_name: ServerName<'static> = "test.com".try_into().unwrap();
let client_config = Arc::new(client_config);
let connector = TlsConnector::new(client_config.clone(), server_name).unwrap();

// 连接到服务器
let stream = async_net::TcpStream::connect((Ipv4Addr::LOCALHOST, 4443)).await.unwrap();

// 创建 TlsStream 并完成握手
let mut stream = connector.connect(stream);
stream.flush().await.unwrap();
```

如需一个完整可运行的示例，请参阅 `examples/` 目录中的代码。

## <a name="文件结构-chinese"></a>文件结构

```
/
├── Cargo.toml       # 项目元数据和依赖项
├── src/
│   ├── lib.rs       # 核心库代码
│   └── test.rs      # 集成测试
├── examples/
│   └── server_client.rs # 可运行的使用示例
└── cert/            # 用于生成测试证书的脚本和配置
```

## <a name="相关故事-chinese"></a>相关故事

本项目所依赖的 `rustls` 库，其诞生在一定程度上是为了响应业界对一个更现代、更内存安全的 OpenSSL 替代品的需求。2014 年，OpenSSL 中的 "Heartbleed"（心脏出血）漏洞极大地改变了互联网的安全格局。该漏洞允许攻击者读取服务器内存，暴露了私钥、密码和用户个人信息等敏感数据。它深刻地提醒了人们，在关键安全软件中，内存安全问题（如缓冲区溢出读取）是多么危险。

`rustls` 利用 Rust 语言的内存安全保证，从根本上消除了包括导致 Heartbleed 的那类 bug。通过建立在 `rustls` 之上，`rustls_async` 继承了这些安全优势，为 Rust 中的异步应用程序构建了一个更安全的生态系统。