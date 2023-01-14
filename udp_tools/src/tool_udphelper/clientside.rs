use std::time::Duration;
use tokio::time::timeout;
use super::*;

/// Provides simple client implementation over UdpSocket
pub struct ClientSide{
    socket: UdpSocket,
    is_connected: bool,
    buff: Vec<u8>,
}

impl ClientSide {
    /// Returns `Result<ClientSide>` if specified address is available
    /// # Example
    /// ```rust
    /// # use udp_tools::ClientSide;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// let client_v1 = 
    ///     ClientSide::new_with_address(Auto).await; 
    /// let client_v2 = 
    ///     ClientSide::new_with_address(Manual(String::from("127.0.0.1:51432") )).await; 
    /// # });
    /// ```
    pub async fn new_with_address(client_addr:AddressSelection) -> Result<Self>{
        let socket = match client_addr{
            AddressSelection::Auto => UdpSocket::bind("0.0.0.0:0").await?,
            AddressSelection::Manual(addr) => UdpSocket::bind(addr).await?,
        };

        let client = ClientSide { socket, is_connected: false, buff: vec![0u8; 1024] };

        Ok(client)
    }

    /// Returns connection status with specified server
    /// 
    /// Status is `true` only if `ClientSide` got an 
    /// `Ok` response from the server on `Ping` request
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8088".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// # server.start();
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// 
    /// client.set_server(serv_addr).await.unwrap();
    /// assert_eq!(client.connection_status(), true);
    /// # });
    /// ```
    pub fn connection_status(&self) -> bool{ self.is_connected }

    /// Returns client's `SocketAddr`
    pub fn local_addr(&self) -> SocketAddr{
        self.socket.local_addr().unwrap()
    }

    /// Returns nothing if the connection is successful
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8089".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// # server.start();
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// 
    /// client.set_server(serv_addr).await.unwrap();
    /// assert_eq!(client.connection_status(), true);
    /// # });
    /// ```
    pub async fn set_server(&mut self, server_addr: AddressSelection) -> Result<()> {
        self.is_connected=true;
        
        self.socket.connect(match server_addr{
            AddressSelection::Auto => "0.0.0.0:0".to_string(),
            AddressSelection::Manual(addr) => addr,
        }).await?;
        
        if let Err(e) = self.ping_server().await{
            self.is_connected=false;

            return Err(e);
        };

        Ok(())
    }

    async fn ping_server(&mut self) -> Result<()>{
        let packet = 
            UdpPacket::new_with_request(PacketRequest::Ping);

        
        let packet = timeout(
            Duration::from_secs(1), 
            self.send_and_recv(packet) ).await??;

        if packet.response() != PacketResponse::Ok {
            return Err(Error::new(ErrorKind::ConnectionRefused, "Ping is not OK!"));
        }

        Ok(())
    }

    /// Sends the provided `UdpPacket` and receives the server's response
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8090".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// # server.start();
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_data(b"unused data");
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// client.set_server(serv_addr).await.unwrap();
    /// 
    /// let packet = client.send_and_recv(packet).await.unwrap();
    /// assert_eq!(packet.request(), PacketRequest::Ping);
    /// assert_eq!(packet.response(), PacketResponse::Ok);
    /// assert_eq!(packet.try_retrieve_data(), Ok(b"unused data".to_vec()));
    /// //Server didn't set any data inside the packet
    /// # });
    /// ```
    pub async fn send_and_recv(&mut self, packet: UdpPacket) -> Result<UdpPacket>{
        if !self.is_connected{
            return Err(Error::new(ErrorKind::AddrNotAvailable , "Not connected to any server!"));
        }
        
        let packet = packet.to_bytes();

        self.socket.send(&packet).await?;

        let n = self.socket.recv( &mut self.buff).await?;

        let packet : UdpPacket = self.buff[..n].to_vec().into();

        Ok(packet)
    }
}