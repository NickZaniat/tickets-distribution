use std::sync::{Arc, RwLock};
use super::*;

/// Implements distributor's role in the process of acquiring tickets
pub struct Distributor{
    udp_server: ServerSide,
    db_storage: Arc<RwLock<Vec<FlightDB>>>,  //available flights, sold flights and tickets
}

impl Distributor{
    /// Returns `Result<Passenger>` if socket binding was successful
    pub async fn new() -> Result<Self>{
        let mut distr = Distributor{
            udp_server: ServerSide::new_with_address(Auto).await?,
            db_storage: RwLock::new( Vec::new() ).into(),
        }; 

        distr.set_logic_fn();
        distr.udp_server.start();
        
        Ok(distr)
    }

    /// Returns `Result<Passenger>` if socket binding was successful
    pub async fn new_with_address(addr: String) -> Result<Self>{
        let mut distr = Distributor{
            udp_server: ServerSide::new_with_address( Manual(addr) ).await?,
            db_storage: RwLock::new( Vec::new() ).into(),
        }; 

        distr.set_logic_fn();
        distr.udp_server.start();
        
        Ok(distr)
    }

    fn set_logic_fn(&mut self) {
        let logic_fn = self.make_logic_fn();

        self.udp_server.set_processing_fn(logic_fn);
    }

    /// Returns socket address
    pub fn get_address(&self) -> String{
        self.udp_server.local_addr().to_string()
    }

    /// Returns `Arc<RwLock<Vec<FlightDB>>>` which  work as storage of flights
    pub fn db_storage(&self) -> Arc<RwLock<Vec<FlightDB>>>{
        self.db_storage.clone()
    }


    /// Creates new flight with rows of seats in range of `1..=42`
    /// 
    /// Variable `rows` will be shifted to the closest value in the range if it is not
    pub async fn gen_fake_flight(&self, rows: u8){
        let mut lock_db = self.db_storage.write().unwrap();

        let mut num = 1;
        if let Some(max_num) = 
                lock_db
                    .iter()
                    .map(|f| f.info.num)
                    .max(){
            num+=max_num;
        };

        let rows = match rows{
            43.. => 42,
            0=> 1,
            other=>other,
        };
        
        let mut seats = Vec::new();
        for row in 1..=rows{
            for letter in "ABCDEF".chars(){
                seats.push(format!("{}{}",letter,row));
            }
        }
        
        lock_db.push(FlightDB{
            info: FlightInfo { num: num, seats_num: rows*6 },
            seats
        });
    }

    fn make_logic_fn(&mut self) -> impl Fn(UdpPacket) -> UdpPacket {
        let shared_arc = self.db_storage.clone();

        let closure = move | packet:UdpPacket | -> UdpPacket {
            let db_storage = shared_arc.clone();

            match packet.request(){
                PacketRequest::Ping => packet.set_response(PacketResponse::Ok),
                PacketRequest::GetFlights => {
                    let lock = db_storage.read().unwrap();

                    let flights: Vec<FlightInfo> = lock.iter().map(|f| f.info ).collect();

                    let packet = packet
                        .set_response(PacketResponse::Ok)
                        .set_data(&convert_to_bytes(&flights));

                    packet
                },
                PacketRequest::RequestTicket => {
                    let flight_num = u32::from_le_bytes( packet.try_retrieve_data().unwrap()[0..4].try_into().unwrap() );

                    let mut lock = db_storage.write().unwrap();

                    if let Some(flight) = lock.iter_mut().find(|f| (f.info.num == flight_num && f.info.seats_num > 0) ){

                        let ticket = flight.seats.pop().unwrap();

                        flight.info.seats_num-=1;

                        let packet = packet
                            .set_response(PacketResponse::Ok)
                            .set_data(ticket.as_bytes());

                        return packet;
                    }
                    else{
                        let packet = packet
                            .set_response(PacketResponse::TicketsSold);

                        return packet;
                    }
                },
            }
        };
        closure
    }
}

/// Used for storing info about flights
#[derive(Clone)]
pub struct FlightDB{
    /// Flight info
    pub info: FlightInfo,
    /// Available seats 
    pub seats: Vec<String>,
}