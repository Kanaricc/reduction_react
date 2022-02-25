use reduction_react::Reactor;

fn main(){
    let reactor=Reactor::new("hello","1.0.4","http://127.0.0.1:8080/hello.json");
    if let Err(e)=reactor.oneclick(){
        println!("failed to check updates: {}",e);
    }

    println!("hello from 1.0.4");
}