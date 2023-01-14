use std::io::{Error, ErrorKind};
use super::*;

/// Implements passenger's role in the process of acquiring tickets
pub struct Passenger{
    udp_client: ClientSide,
    acquired_tickets: Vec<FlightTicket>
}

impl Passenger{
    /// Returns `Result<Passenger>` if socket binding was successful
    pub async fn new() -> Result<Self>{
        Ok(Passenger { 
            udp_client: ClientSide::new_with_address(Auto).await?, 
            acquired_tickets: vec![], 
        })
    }

    /// Returns all previosly acquired tickets
    pub fn acquired_tickets(&self) -> Vec<FlightTicket> {self.acquired_tickets.to_vec()}

    /// Attemp to ping specified distributor address
    pub async fn try_connect(&mut self, serv_addr: &String) -> Result<()>{
        self.udp_client.set_server(Manual( serv_addr.to_owned() )).await
    }

    /// Fetch available flights from the distributor
    pub async fn fetch_flights(&mut self) -> Result<Vec<FlightInfo>>{
        let packet = UdpPacket::new_with_request(PacketRequest::GetFlights);

        let packet = self.udp_client.send_and_recv(packet).await?;
        
        match packet.response() {
            PacketResponse::Ok => {
                let data = packet.try_retrieve_data().unwrap();

                Ok(convert_to_flightinfo(&data))
            },
            _ => Err(Error::new(ErrorKind::InvalidData, "Response is invalid")),
        }
    }

    /// Query a ticket from distributor
    /// 
    /// Returns `None` if there is no tickets are available for this flight
    pub async fn query_ticket_for_a_flight(&mut self, flight_num: u32) -> Result<Option<String>>{
        let packet = 
            UdpPacket::new_with_request(PacketRequest::RequestTicket)
                .set_data(&flight_num.to_le_bytes());

        let packet = self.udp_client.send_and_recv(packet).await?;
        
        match packet.response() {
            PacketResponse::None | PacketResponse::TicketsSold => Ok(None),
            PacketResponse::Ok => {
                let ticket = String::from_utf8( packet.try_retrieve_data().unwrap() ).unwrap();

                self.acquired_tickets.push(FlightTicket{
                    flight_num,
                    ticket: ticket.clone()
                });

                Ok(Some(ticket))
            },
            _ => Err(Error::new(ErrorKind::InvalidData, "Response is invalid")),
        }
    }
}

/// Used for saving acquired tickets on passenger side
#[derive(Clone)]
pub struct FlightTicket{
    pub flight_num: u32,
    pub ticket: String, // [A-F][1-99]
}