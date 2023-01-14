//! `Passenger` and `Distributor` implementation using `udp_tools` crate

pub use pass::Passenger;
pub use distr::Distributor;


mod distr;
mod pass;

use serde::{Deserialize,Serialize};
use std::io::Result;
use AddressSelection::*;
use udp_tools::*;


/// Used as transmitted info in packets
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FlightInfo{
    /// Flight number
    pub num: u32,
    /// Number of seats. 
    /// Value can be up to 42*6 (42 rows with ABCDEF seats)
    pub seats_num: u8,
}

/// Converts vector of structs FlightInfo to vector of bytes.
/// 
/// For converting serde_cbor is used
/// # Example
/// ```rust
/// # use udp_ticket_distribution::convert_to_flightinfo;
/// # use udp_ticket_distribution::convert_to_bytes;
/// # use udp_ticket_distribution::FlightInfo;
/// let mut data = Vec::new();
/// 
/// for i in 1..10{
///     data.push(FlightInfo{ num: i, seats_num: 42*6});
/// }
/// 
/// let serialized_data: Vec<u8> = convert_to_bytes(&data);
/// let deserialized_data: Vec<FlightInfo> = convert_to_flightinfo(&serialized_data);
/// 
/// std::iter::zip(data, deserialized_data).map( | (d,dd) |{
///     assert_eq!(d.num, dd.num);
///     assert_eq!(d.seats_num, dd.seats_num);
/// }).collect::<Vec<_>>();
/// ```
pub fn convert_to_bytes(data: &Vec<FlightInfo>) -> Vec<u8>{
    let mut bytearr =  Vec::new(); 

    serde_cbor::to_writer(&mut bytearr, &data).unwrap();

    bytearr
}

/// Converts vector of bytes to vector of FlightInfo structs.
/// 
/// For converting serde_cbor is used
/// # Example
/// ```rust
/// # use udp_ticket_distribution::convert_to_flightinfo;
/// # use udp_ticket_distribution::convert_to_bytes;
/// # use udp_ticket_distribution::FlightInfo;
/// let mut data = Vec::new();
/// 
/// for i in 1..10{
///     data.push(FlightInfo{ num: i, seats_num: 42*6});
/// }
/// 
/// let serialized_data: Vec<u8> = convert_to_bytes(&data);
/// let deserialized_data: Vec<FlightInfo> = convert_to_flightinfo(&serialized_data);
/// 
/// std::iter::zip(data, deserialized_data).map( | (d,dd) |{
///     assert_eq!(d.num, dd.num);
///     assert_eq!(d.seats_num, dd.seats_num);
/// }).collect::<Vec<_>>();
/// ```
pub fn convert_to_flightinfo(data: &Vec<u8>) -> Vec<FlightInfo>{
    serde_cbor::from_reader::<Vec<FlightInfo>, _>(data.as_slice()).unwrap()
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn udp_socket_distribution_converting_test(){
        let mut data = Vec::new();

        for i in 1..10{
            data.push(FlightInfo{ num: i, seats_num: 42*6});
        }

        let serialized_data = convert_to_bytes(&data);

        let deserialized_data = convert_to_flightinfo(&serialized_data);

        let iter = std::iter::zip(data, deserialized_data).map( | (d,dd) |{
            assert_eq!(d.num, dd.num);
            assert_eq!(d.seats_num, dd.seats_num);
        });

        iter.collect()
    }

    #[tokio::test]
    async fn udp_socket_distribution_communicating_test() -> std::io::Result<()>{
        use super::distr::FlightDB;
        use std::sync::Arc;
        use futures::lock::Mutex;
        //serverside
        let distr = Distributor::new_with_address("127.0.0.1:8083".to_string()).await?;

        distr.gen_fake_flight(1).await; //1*6 seats for 10 passengers

        let arc = distr.db_storage();
        let lock = arc.read().unwrap();

        let mut flightdb = lock[0].clone();

        drop(lock);
        drop(arc);

        //clients side
        let distr_addr = distr.get_address();

        let mut psngers = Vec::new();
        for _ in 0..10{
            let mut p: Passenger = Passenger::new().await?;
            p.try_connect(&distr_addr).await.unwrap();
            psngers.push(Arc::new(Mutex::new(p)));
        };

        let info;

        {
            let mut guard = psngers[0].lock().await;
            info = guard.fetch_flights().await?;
            //guard drops here
        }

        assert_eq!(info.len(), 1);

        let mut psngers_flightdb = FlightDB{
            info: info[0],
            seats: Vec::new(),
        };

        let mut tasks = Vec::new();

        for i in 0..10{
            let client = psngers[i].clone();
            let data_num = psngers_flightdb.info.num.clone();
            tasks.push(tokio::spawn(async move {
                client.lock().await.
                    query_ticket_for_a_flight(data_num).await.unwrap()
            }));
        };

        //tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mut none_count = 0;

        for fut in tasks.into_iter(){
            if let Some(ticket) = fut.await?{
                psngers_flightdb.seats.push(ticket);
            }
            else{
                none_count+=1;
            }
        };

        assert_eq!(none_count, 4);

        flightdb.seats.sort();
        psngers_flightdb.seats.sort();
        psngers_flightdb.info.seats_num = psngers_flightdb.seats.len().try_into().unwrap();

        assert_eq!(flightdb.info.seats_num, psngers_flightdb.info.seats_num);

        let mut iter = std::iter::zip(flightdb.seats, psngers_flightdb.seats).map( | (f,pf) |{
            assert_eq!(f, pf);
        });

        while let Some(_) = iter.next() {}

        Ok(())
    }
}