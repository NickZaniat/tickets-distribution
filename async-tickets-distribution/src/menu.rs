use std::io::Write;
use std::collections::HashMap;
use std::io::{stdin,stdout};
use std::process::exit;
pub struct CLPageOption{
    name: String,
    fn_name_on_choose: String,
    redirect_on_page: String,
}
impl CLPageOption {
    /*pub fn new(name: &str, redirect_on_page: &str) -> Self{
        CLPageOption { 
            name: name.to_owned(), 
            fn_name_on_choose: String::new(), 
            redirect_on_page: redirect_on_page.to_owned()}
    }*/

    pub fn new_with_closure(name: &str, redirect_on_page: &str, fn_name_on_choose: &str ) -> Self{
        CLPageOption { 
            name: name.to_owned(), 
            fn_name_on_choose: fn_name_on_choose.to_owned(), 
            redirect_on_page: redirect_on_page.to_owned()}
    }
}

pub struct CLMenuPage{
    name: String,
    description: String,
    options: Vec<CLPageOption>,
    is_start_menu: bool,
}
impl CLMenuPage {
    pub fn new(name: &str, description: &str, is_start_menu: bool ) -> Self{
        CLMenuPage { 
            name: name.to_owned(), 
            description: description.to_owned(), 
            options:Vec::new(), 
            is_start_menu }
    }
    pub fn add(&mut self, option: CLPageOption){
        self.options.push(option);
    }
}

pub struct CLMenu{
    stdin: std::io::Stdin,
    stdout: std::io::Stdout,
    menus: HashMap<String,CLMenuPage>,
    start_menu: String,
    current_menu: String,
}

impl CLMenu {
    pub fn new() -> Self{
        CLMenu {
            stdin: stdin(),
            stdout: stdout(),
            menus: HashMap::new(),
            start_menu: String::new(),
            current_menu: String::new(),
        }
    }

    pub fn add_menu(&mut self, menu: CLMenuPage){
        if menu.is_start_menu { self.start_menu= menu.name.clone(); }
        self.menus.insert(menu.name.clone(), menu);
    }

    pub fn start(&mut self) -> String {
        if self.start_menu.is_empty() {
        eprintln!("Start menu not set..."); 
        exit(1);
        }

        self.current_menu = self.start_menu.clone();

        self.show_menu()
    }

    fn show_menu(&mut self) -> String{
        clearscreen::clear().expect("Error while clearing screen");
        let mut handle = self.stdout.lock();
        
        let menu = self.menus.get(&self.current_menu);

        let menu = match menu{
            Some(m) => m,
            None => { 
                eprintln!("Menu not found! Program is closed."); 
                exit(2); 
            }
        };

        writeln!(handle, "{}", menu.description).expect("Error while writing to stdout.");
        for i in 1..=menu.options.len(){
            writeln!(handle, "{:>2}. {}",i,menu.options[i-1].name).expect("Error while writing to stdout..");
        }
        write!(handle, "input (number): ").expect("Error while writing to stdout...");
        handle.flush().expect("Flush error");

        self.input_response()
    }

    fn input_response(&mut self) -> String{
        let menu = self.menus.get_mut(&self.current_menu).unwrap();
        let variant;
        
        loop {
            let mut input = String::new();
            self.stdin.read_line(&mut  input).expect("Error while reading input");

            let vrnt= input.split_whitespace().next();
            let Some(vrnt) = vrnt else {continue;};
            let Ok(v) = vrnt.parse::<usize>() else {continue;};

            if  (1..=menu.options.len()).contains(&v) {
                variant = v;
                break;
            }

            eprintln!("Invalid input. Please choose from 1 to {}", menu.options.len());
            print!("input: ");
            self.stdout.flush().expect("Error while flushing after invalid input");
        }

        self.current_menu = menu.options[variant-1].redirect_on_page.clone();

        menu.options[variant-1].fn_name_on_choose.clone()
    }
}