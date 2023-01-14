pub use self::clientside::ClientSide;
pub use self::serverside::ServerSide;

mod clientside;
mod serverside;

use tokio::net::UdpSocket;
use std::io::{Result, Error, ErrorKind};
use super::tool_udppacket::*;
use std::net::SocketAddr;

/// Holds two input variants of address
/// 
/// Manual variant requires "ip:port" string
/// 
/// Ex: `Manual("1.2.3.4:12345".to_string())`[^note].
/// 
/// [^note]: `"0.0.0.0:0"` is the same as `Auto` variant
/// # Example
/// ```rust
/// # use udp_tools::*;
/// # use udp_tools::AddressSelection::*;
/// # use tokio::runtime::Runtime;
/// # let mut rt = Runtime::new().unwrap();
/// # rt.block_on(async {
/// let server = 
///     ServerSide::new_with_address(
///         Manual("0.0.0.0:8080".to_string())).await;
/// let client = 
///     ClientSide::new_with_address(Auto).await; 
/// # });
/// ``` 
#[derive(Clone)]
pub enum AddressSelection{
    Auto,
    Manual(String),
}

#[cfg(test)]
mod tests{
    use super::*;
    use AddressSelection::*;

    #[tokio::test]
    async fn client_and_server_connect() -> Result<()>{

        let mut server = ServerSide::new_with_address(Manual("0.0.0.0:8080".to_string())).await?;

        server.set_processing_fn(| packet | {
                match packet.request(){
                    PacketRequest::Ping => packet.set_response(PacketResponse::Ok),
                    _ => packet.set_response(PacketResponse::None),
                }
        });
        let server_addr = server.local_addr().to_string();

        server.start();
        
        let mut client = ClientSide::new_with_address(Auto).await?;
        client.set_server(Manual(server_addr)).await?;

        let packet = client.send_and_recv(
                UdpPacket::new_with_request(PacketRequest::Ping)
        ).await?;

        server.stop();

        assert_eq!(packet.response(), PacketResponse::Ok);

        Ok(())
    }

    #[tokio::test]
    async fn client_and_server_data_exchange() -> Result<()>{
        let mut server = ServerSide::new_with_address(Manual("0.0.0.0:8081".to_string())).await?;

        server.set_processing_fn(| packet | {
            match packet.request(){
                    PacketRequest::Ping => packet.set_response(PacketResponse::Ok),
                    PacketRequest::GetFlights => {

                        assert_eq!(&packet.try_retrieve_data().unwrap(), b"GetFlights");

                        packet
                            .set_response(PacketResponse::Ok)
                            .set_data(b"Flights")
                    }
                    _ => packet.set_response(PacketResponse::None),
                }
        });
        let server_addr = server.local_addr().to_string();

        server.start();
        
        let mut client = ClientSide::new_with_address(Auto).await?;
        client.set_server(Manual(server_addr)).await?;

        let packet = client.send_and_recv(
                UdpPacket::new_with_request(PacketRequest::GetFlights)
                    .set_data(b"GetFlights")
        ).await?;

        assert_eq!(packet.response(), PacketResponse::Ok);
        assert_eq!(&packet.try_retrieve_data().unwrap(), b"Flights");

        server.stop();

        assert_eq!(packet.response(), PacketResponse::Ok);

        Ok(())
    }
}