use reduction_react::Reactor;

fn main(){
    let reactor=Reactor::new("hello","1.0.3","http://127.0.0.1:8080/hello.json");
    reactor.self_update_if_available().unwrap();
    reactor.check_update_and_update().unwrap();
    reactor.self_update_if_available().unwrap();
}