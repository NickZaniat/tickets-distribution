//! CLI app for demonstrating client/server side

use udp_ticket_distribution::*;
use std::io::{stdin, stdout, Write};
use menu::*;

#[doc(hidden)]
mod menu;


#[doc(hidden)]
#[tokio::main]
async fn main() {
    let mut menu = build_menu();
    loop {
        match menu.start().as_str(){
            "client_mode" => client_mode().await,
            "server_mode" => server_mode().await,
            "exit" => return,
            _ => (),
        }// async Fn traits not implemented as needed, using this workaround
    }
}

#[doc(hidden)]
fn build_menu() -> CLMenu{
    let mut menu = CLMenu::new();

    let mut menu_main_page = CLMenuPage::new(
        "Main",
        "Please, choose your role:",
        true,
    );
    menu_main_page.add(CLPageOption::new_with_closure("Passenger", "Main", "client_mode"));
    menu_main_page.add(CLPageOption::new_with_closure("Distributor", "Main", "server_mode"));
    menu_main_page.add(CLPageOption::new_with_closure("Exit", "", "exit"));
    menu.add_menu(menu_main_page);

    menu
}

#[doc(hidden)]
async fn client_mode(){
    clearscreen::clear().unwrap();
    //welcome
    println!("You are a Passenger!");
    //create client
    let mut psngr = Passenger::new().await.unwrap();
    //input ip:port of the server 
    print!("Please, input your distributor ip:port numbers (Ex: 127.0.0.1:8080) or simple type \"exit\"\nThis input is everything sensitive\n input: ");
    stdout().flush().unwrap();
    loop {
        let mut serv_addr = String::new();
        stdin().read_line(&mut serv_addr).unwrap();
        serv_addr.pop();//trailing newline removed
        
        if &serv_addr == "exit" {return ;}
        if psngr.try_connect(&serv_addr).await.is_ok(){
            println!("Successful connection!");
            break;
        }
        println!("Connection failed. Maybe a typoo?");
    }
    // (loop)
    //input op(["fetch", "flights"], ["ticketfor", "flight_number"], ["exit"])
    //send recv print
    loop {
        print!("What do you want to do? (fetch flights | ticketfor [flight_number] | exit) \n input: ");
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let input: Vec<&str> = input.split_whitespace().collect();
        match input[..]{
            ["fetch","flights"] => {
                let data = psngr.fetch_flights().await;
                if let Err(e) = data {
                    eprintln!("Error: {}", e.to_string());
                    //sleep?
                    continue;
                }
                println!("Recieved data:");
                for flight in data.unwrap(){
                    println!("Flight: {:3} | seats: {}",flight.num, flight.seats_num);
                }
                println!("End of recieved data.");
            },
            ["ticketfor", flight_number] => {
                let flight_number: u32 = match flight_number.parse(){
                    Ok(v) => v,
                    Err(_) =>{
                        println!("Invalid input. Maybe a typoo? (Ex: ticketfor 1)");
                        continue;
                    }
                };

                let data = psngr.query_ticket_for_a_flight(flight_number).await;
                if let Err(e) = data {
                    eprintln!("Error: {}", e.to_string());
                    //sleep?
                    continue;
                }

                match data.unwrap(){
                    Some(t) => println!("Recieved ticket {} for a flight {}",t,flight_number),
                    None => println!("Ticket did not received. Check if flight is still available."),
                }
            },
            ["exit"] => break,
            _ => println!("Invalid input. Maybe a typoo?"),
        }
    }
}

#[doc(hidden)]
async fn server_mode(){
    clearscreen::clear().unwrap();
    //welcome
    println!("You are a Distributor!");
    
    let distr;
    //input ip:port of the server
    print!("Please, input your desired ip:port numbers (Ex: 127.0.0.1:8080) or simple type \"exit\"\nThis input is everything sensitive\n input: ");
    stdout().flush().unwrap();
    loop {
        let mut my_addr = String::new();
        stdin().read_line(&mut my_addr).unwrap();
        my_addr.pop();//trailing newline removed
        
        if &my_addr == "exit" {return ;}
        if let Ok(d) = Distributor::new_with_address(my_addr).await{
            println!("Successful ip:port binding!");
            distr = d;
            break;
        }
        print!("Binding failed. Maybe a typoo?\n input: ");
        stdout().flush().unwrap();
    }
    // (loop)
    //start + logging|stop|exit
    println!("Server started!");
    loop {
        print!("What do you want to do? ( see flights | genflight [1..42] | exit) \n input: ");
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let input: Vec<&str> = input.split_whitespace().collect();
        match input[..]{
            ["see", "flights"] => {
                println!("Flights info:");
                for f in distr.db_storage().read().unwrap().iter(){
                    println!("Flight: {:3}, seats: {:3}",f.info.num, f.info.seats_num)
                }
                println!("Flights info end.");
            },
            ["genflight", seats_rows] => {
                let seats_rows: u8 = match seats_rows.parse(){
                    Ok(v) => v,
                    Err(_) =>{
                        println!("Invalid input. Maybe a typoo? (Ex: genflight 1)");
                        continue;
                    }
                };

                distr.gen_fake_flight(seats_rows).await;

                println!("Flight created! Now you can find him with command \"see flights\".");
            },
            ["exit"] => break,
            _ => println!("Invalid input. Maybe a typoo?"),
        }
    }
}