mod tool_packetdata;

use serde::{Serialize,Deserialize};
use serde_repr::{Serialize_repr,Deserialize_repr};

/// Provides simple packet creation and setup
/// 
/// For data verification CRC-16/IBM-SDLC is used
/// 
/// Request and response fields implemented as `enum`
/// and contain basic required types for `udp_ticket_distribution` crate
#[derive(Serialize,Deserialize)]
pub struct UdpPacket{
    secret:u8,
    request:PacketRequest,
    response: PacketResponse,
    data: Option<tool_packetdata::PacketData>,
}

impl UdpPacket{
    /// Creates new `UdpPacket` with specified request
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping);
    /// 
    /// assert_eq!(packet.request(), PacketRequest::Ping);
    /// ```
    pub fn new_with_request( request:PacketRequest) -> UdpPacket{
        UdpPacket{
            secret: 51,
            request,
            response: PacketResponse::None, 
            data: None}
    }

    /// Returns `UdpPacket` with specified data 
    /// 
    /// For data verification on reciever side 
    /// packet also saves crc
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_data(b"ignored data");
    /// 
    /// assert_eq!(packet.try_retrieve_data(), Ok(b"ignored data".to_vec()));
    /// ```
    pub fn set_data(mut self ,data: &[u8]) -> UdpPacket{
        use tool_packetdata::*;
        self.data= Some(PacketData::new_with_data(data));
        self
    }

    /// Returns `Result<Vec<u8>>` with packet data
    /// if newly calculated crc is equal to crc in packet
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_data(b"ignored data");
    /// 
    /// assert_eq!(packet.try_retrieve_data(), Ok(b"ignored data".to_vec()));
    /// ```
    pub fn try_retrieve_data(&self) -> Result<Vec<u8>,String>{
        if self.secret != 51 {return Err("Secret is invalid!".to_string());}
        
        self.data
            .as_ref()
            .ok_or("Data is None!".to_string())?
            .try_retrieve_data().map(|data| data.to_vec())
    }

    /// Returns `UdpPacket` with specified response
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_response(PacketResponse::None);
    /// 
    /// assert_eq!(packet.response(), PacketResponse::None);
    /// ```
    pub fn set_response(mut self ,response: PacketResponse) -> UdpPacket{
        self.response = response;
        self
    }

    /// Returns `PacketResponse` of the packet
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_response(PacketResponse::None);
    /// 
    /// assert_eq!(packet.response(), PacketResponse::None);
    /// ```
    pub fn response(&self) -> PacketResponse { self.response.clone() }  

    /// Returns `PacketRequest` of the packet
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping);
    /// 
    /// assert_eq!(packet.request(), PacketRequest::Ping);
    /// ```
    pub fn request(&self) -> PacketRequest { self.request.clone() }

    /// Converts packet to bytes vector
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    ///     .set_response(PacketResponse::None)
    ///     .set_data(b"some data");
    /// 
    /// let byte_array = packet.to_bytes();
    /// 
    /// let packet_from_bytes: UdpPacket = byte_array.into();
    /// 
    /// assert_eq!(packet_from_bytes.request(),             
    ///     packet.request());
    /// assert_eq!(packet_from_bytes.response(),            
    ///     packet.response());
    /// assert_eq!(packet_from_bytes.try_retrieve_data(),   
    ///     packet.try_retrieve_data());
    /// ```
    pub fn to_bytes(&self) -> Vec<u8>{
        //let mut bytearr =  vec![0,0,0,0]; //4 bytes for u32 max size // useless, tokio udpsocket peek method raises error on windows if buffer is less then required
        
        let mut bytearr =  Vec::new(); 

        serde_cbor::to_writer(&mut bytearr, &self).unwrap();

        /*
        let size: u32 = bytearr.len().try_into().unwrap(); //can panic if packet more than 2^32 bytes which is more than udp can handle

        let size = size.to_le_bytes(); //win&lin, 4 bytes 

        for i in 0..4 {
            bytearr[i]=size[i];
        }
        */
        
        bytearr
    }
}   

impl From<Vec<u8>> for UdpPacket {
    /// Creates packet from bytes vector
    /// # Example
    /// ```rust
    /// # use udp_tools::*;
    /// # let packet = UdpPacket::new_with_request(PacketRequest::Ping)
    /// #    .set_response(PacketResponse::None)
    /// #    .set_data(b"some data");
    /// #
    /// # let byte_vec = packet.to_bytes();
    /// #
    /// let packet_from_bytes: UdpPacket = byte_vec.into();
    /// 
    /// assert_eq!(packet_from_bytes.request(),             
    ///     packet.request());
    /// assert_eq!(packet_from_bytes.response(),            
    ///     packet.response());
    /// assert_eq!(packet_from_bytes.try_retrieve_data(),   
    ///     packet.try_retrieve_data());
    /// ```
    fn from(item: Vec<u8>) -> Self {
        //serde_cbor::from_reader::<UdpPacket, _>(&item[4..]).unwrap() //skipping first 4 bytes of size
        serde_cbor::from_reader::<UdpPacket, _>(item.as_slice()).unwrap()
    }
}

/// Holds possible client request
#[derive(Debug, PartialEq, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PacketRequest{
    Ping=1,
    GetFlights,
    RequestTicket,
}

/// Holds possible server response
#[derive(Debug, PartialEq, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PacketResponse{
    None,
    Ok,
    ErrorInRequest,
    TicketsSold,
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn tooludppacket_full_test(){
        let data = b"data";
        let udppacket = 
            UdpPacket::new_with_request(PacketRequest::Ping)
                .set_data(data)
                .set_response(PacketResponse::None);

        assert_eq!(udppacket.try_retrieve_data().unwrap(), b"data".to_vec());
        assert_eq!(udppacket.request(), PacketRequest::Ping);
        assert_eq!(udppacket.response(), PacketResponse::None);
    }

    #[test]
    fn tooludppacket_error_test(){
        let udppacket = 
            UdpPacket::new_with_request(PacketRequest::Ping)
            .set_response(PacketResponse::None);

        assert_eq!(udppacket.try_retrieve_data(), Err("Data is None!".to_string()));
    }

    #[test]
    fn tooludppacket_serde(){
        use serde_cbor::{from_reader, to_writer};

        let packet =
            UdpPacket::new_with_request(PacketRequest::Ping)
                .set_data(b"data")
                .set_response(PacketResponse::None);

        let mut aka_file = Vec::new();

        to_writer(&mut aka_file, &packet).unwrap();

        assert_eq!(aka_file, 
            vec![164, 102, 115, 101, 99, 114, 101, 116, 24, 51, 103, 114, 101, 113, 117, 101, 115, 116, 1, 104, 114, 101, 
                115, 112, 111, 110, 115, 101, 0, 100, 100, 97, 116, 97, 162, 99, 99, 114, 99, 25, 173, 108, 100, 100, 97, 
                116, 97, 132, 24, 100, 24, 97, 24, 116, 24, 97]);

        let recieved_packet: UdpPacket = from_reader(aka_file.as_slice()).unwrap();
        
        assert_eq!(recieved_packet.try_retrieve_data().unwrap(), b"data".to_vec());
        assert_eq!(recieved_packet.request, PacketRequest::Ping);
        assert_eq!(recieved_packet.response, PacketResponse::None);
    }

    #[test]
    fn tooludppacket_from_and_into(){
        let packet =
            UdpPacket::new_with_request(PacketRequest::Ping)
                .set_data(b"a lot of data somewhere there in a packet")
                .set_response(PacketResponse::None);

        let bytearr = packet.to_bytes();

        /*
        assert_eq!(bytearr.len(), 135);

        assert_eq!(bytearr, 
            vec![135, 0, 0, 0, //4 bytes of size (131 element + 4 bytes of size)
            164, 102, 115, 101, 99, 114, 101, 116, 24, 51, 103, 114, 101, 113, 117, 101, 115, 116, 1, 104, 114, 101, 115, 112, 111, 110, 115, 101, 0, 100, 100, 
            97, 116, 97, 162, 99, 99, 114, 99, 25, 164, 222, 100, 100, 97, 116, 97, 152, 41, 24, 97, 24, 32, 24, 108, 24, 111, 24, 116, 24, 32, 24, 111, 24, 
            102, 24, 32, 24, 100, 24, 97, 24, 116, 24, 97, 24, 32, 24, 115, 24, 111, 24, 109, 24, 101, 24, 119, 24, 104, 24, 101, 24, 114, 24, 101, 24, 32, 24, 
            116, 24, 104, 24, 101, 24, 114, 24, 101, 24, 32, 24, 105, 24, 110, 24, 32, 24, 97, 24, 32, 24, 112, 24, 97, 24, 99, 24, 107, 24, 101, 24, 116]);
        */

        assert_eq!(bytearr.len(), 131);

        assert_eq!(bytearr, 
            vec![
            164, 102, 115, 101, 99, 114, 101, 116, 24, 51, 103, 114, 101, 113, 117, 101, 115, 116, 1, 104, 114, 101, 115, 112, 111, 110, 115, 101, 0, 100, 100, 
            97, 116, 97, 162, 99, 99, 114, 99, 25, 164, 222, 100, 100, 97, 116, 97, 152, 41, 24, 97, 24, 32, 24, 108, 24, 111, 24, 116, 24, 32, 24, 111, 24, 
            102, 24, 32, 24, 100, 24, 97, 24, 116, 24, 97, 24, 32, 24, 115, 24, 111, 24, 109, 24, 101, 24, 119, 24, 104, 24, 101, 24, 114, 24, 101, 24, 32, 24, 
            116, 24, 104, 24, 101, 24, 114, 24, 101, 24, 32, 24, 105, 24, 110, 24, 32, 24, 97, 24, 32, 24, 112, 24, 97, 24, 99, 24, 107, 24, 101, 24, 116]);

        let recieved_packet: UdpPacket = bytearr.into();
        
        assert_eq!(recieved_packet.try_retrieve_data().unwrap(), b"a lot of data somewhere there in a packet".to_vec());
        assert_eq!(recieved_packet.request, PacketRequest::Ping);
        assert_eq!(recieved_packet.response, PacketResponse::None);
    }
}