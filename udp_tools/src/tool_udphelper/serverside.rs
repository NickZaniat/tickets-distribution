use std::sync::Arc;
use tokio::task::JoinHandle;
use super::*;

type FnTraitAsync = Arc< dyn (Fn(UdpPacket) -> UdpPacket) + Send + Sync >;

fn fn_trait_into_async(func: impl (Fn(UdpPacket) -> UdpPacket) + Send + Sync + 'static) -> FnTraitAsync{
    Arc::new(func) 
}

/// Provides server implementation over UdpSocket
pub struct ServerSide{
    socket: Arc<UdpSocket>,
    is_running: bool,
    loop_handle: Option< JoinHandle< Result<()> > >,
    processing_fn: FnTraitAsync,
}

impl ServerSide {
    /// Returns `Result<ServerSide>` if specified address is available
    /// # Example
    /// ```rust
    /// # use udp_tools::ServerSide;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// let server_v1 = 
    ///     ServerSide::new_with_address(Auto).await; 
    /// let server_v2 = 
    ///     ServerSide::new_with_address(Manual(String::from("127.0.0.1:51433") )).await; 
    /// # });
    /// ```
    pub async fn new_with_address(addr:AddressSelection) -> Result<Self>{
        let socket = match addr{
            AddressSelection::Auto => UdpSocket::bind("0.0.0.0:0").await?,
            AddressSelection::Manual(addr) => UdpSocket::bind(addr).await?,
        };

        let server = ServerSide { 
            socket: socket.into(),
            is_running: false,
            loop_handle: None,
            processing_fn:  fn_trait_into_async( | p | p.set_response(PacketResponse::Ok) )
        };

        Ok(server)
    }

    /// Sets server's logic over recieved `UdpPacket`s
    /// 
    /// Requires server restart to update fn logic
    /// 
    /// If packet response is `None` then packet will not be sent
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8093".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// // Hidden server setup
    /// server.set_processing_fn(|p| p.set_response(PacketResponse::None));
    /// // Server will not respond to any packet (even Ping)
    /// # server.start();
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// 
    /// let error = client.set_server(serv_addr).await;
    /// // Client waits for 1 second then raises TimedOut
    /// assert_eq!(client.connection_status(), false);
    /// assert!(error.is_err());
    /// # });
    /// ```
    pub fn set_processing_fn(&mut self, processing_fn: impl (Fn(UdpPacket) -> UdpPacket) + Send + Sync + 'static){
        self.processing_fn = fn_trait_into_async(processing_fn);
    }

    /// Returns server running status
    pub fn is_running(&self) -> bool{ self.is_running }
    /// Returns server's `SocketAddr`
    pub fn local_addr(&self) -> SocketAddr{
        self.socket.local_addr().unwrap()
    }

    /// Server will start recieving incoming packets
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8094".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// // Hidden server setup
    /// server.set_processing_fn(|p| p.set_response(PacketResponse::None));
    /// // Server will not respond to any packet (even Ping)
    /// server.start();
    /// 
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// 
    /// client.set_server(serv_addr.clone()).await; //TimedOut
    /// assert_eq!(client.connection_status(), false);
    /// 
    /// server.stop();
    /// server.set_processing_fn(|p| p.set_response(PacketResponse::Ok));
    /// server.start();
    /// 
    /// client.set_server(serv_addr).await.unwrap();
    /// assert_eq!(client.connection_status(), true);
    /// # });
    /// ```
    pub fn start(&mut self){
        self.stop();

        self.loop_handle = Some(
            tokio::spawn(
                Self::loop_recv(
                    self.socket.clone(), 
                    self.processing_fn.clone()))
        );

        self.is_running = true;
    }

    /// Server will stop recieving incoming packets
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # use udp_tools::AddressSelection::*;
    /// # use tokio::runtime::Runtime;
    /// # let mut rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// # let serv_addr = Manual("127.0.0.1:8095".to_string());
    /// # let mut server = 
    /// #     ServerSide::new_with_address(serv_addr.clone()).await.unwrap();
    /// // Hidden server setup
    /// server.set_processing_fn(|p| p.set_response(PacketResponse::Ok));
    /// server.start();
    /// 
    /// let mut client = 
    ///     ClientSide::new_with_address(Auto).await.unwrap(); 
    /// 
    /// server.stop();
    /// 
    /// client.set_server(serv_addr.clone()).await; //TimedOut
    /// assert_eq!(client.connection_status(), false);
    /// 
    /// server.start();
    /// 
    /// client.set_server(serv_addr).await.unwrap();
    /// assert_eq!(client.connection_status(), true);
    /// # });
    /// ```
    pub fn stop(&mut self){
        if let Some(handle) = &self.loop_handle {
            if !handle.is_finished() { handle.abort()}
        }

        self.is_running = false;
    }

    async fn process_recieved_packet(socket: Arc<UdpSocket>, data:Vec<u8>, addr: SocketAddr, processing_fn: FnTraitAsync ) -> Result<()>{
        let packet: UdpPacket = data.into();
        let packet = processing_fn(packet);

        if packet.response() != PacketResponse::None{
            socket.send_to(&packet.to_bytes(), addr).await?;
        }
        Ok(())
    }

    async fn loop_recv(socket: Arc<UdpSocket>, processing_fn: FnTraitAsync ) -> Result<()>{
        let mut buff = vec![0u8; 1024];
        loop {
            let (n, addr) = socket.recv_from(&mut buff).await?;

            tokio::task::spawn(
                ServerSide::process_recieved_packet(socket.clone(), buff[..n].to_vec(), addr, processing_fn.clone())
            );
        }
    }
}