use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize)]
pub struct PacketData{
    crc: u16,
    data: Vec<u8>,
}

fn make_crc( data: &Vec<u8>) -> u16 {
    use crc::{Crc,CRC_16_IBM_SDLC};
    let crc = Crc::<u16>::new(&CRC_16_IBM_SDLC);
    crc.checksum(data)
}

impl PacketData {
    pub fn new_with_data(data: &[u8])-> PacketData{
        let data = data.to_vec();//copy
        let packet = PacketData {
            crc: make_crc(&data),
            data,
        };
        packet
    }

    pub fn try_retrieve_data(&self) -> Result<&Vec<u8>, String> {
        let crc = make_crc(&self.data);

        match self.crc == crc {
            true => {
                Ok(&self.data)
            },
            false => Err("Crc error!".to_string())
        }

    }
}


#[cfg(test)]
mod tests{
    use super::PacketData;

    #[test]
    fn toolpacketdata_simple_test(){
        let mut data = b"array".to_vec();
        let packet = PacketData::new_with_data(&data);

        data[0]=b'b';
        assert_ne!(data, packet.data);
    }

    #[test]
    fn toolpacketdata_bad_data(){
        let mut packet = PacketData::new_with_data(&[1;100]);
        packet.data[0]=5;
        assert_eq!(packet.try_retrieve_data(),Err("Crc error!".to_string()));
    }

    #[test]
    fn toolpacketdata_retrieving_data(){
        let packet = PacketData::new_with_data(b"send data");
        assert_eq!( packet.try_retrieve_data().unwrap(), b"send data");
    }

    #[test]
    fn toolpacketdata_serde(){
        use serde_cbor::{from_reader, to_writer};

        let packet = PacketData::new_with_data(b"cbor test");

        let mut aka_file = Vec::new();

        to_writer(&mut aka_file, &packet).unwrap();

        assert_eq!(aka_file, 
            vec![162, 99, 99, 114, 99, 25, 69, 53, 100, 100, 97, 116, 97, 137, 24, 
            99, 24, 98, 24, 111, 24, 114, 24, 32, 24, 116, 24, 101, 24, 115, 24, 116]);

        let recieved_packet: PacketData = from_reader(aka_file.as_slice()).unwrap();
        
        assert_eq!(packet.data, recieved_packet.data);
    }
}