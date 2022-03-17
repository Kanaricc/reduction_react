use reduction_react::ReactorBuilder;

fn main() {
    let reactor = ReactorBuilder::new()
        .name("hello")
        .version("1.0.4")
        .publishing_url("http://127.0.0.1:8080/hello.json")
        .finish();
    if let Err(e) = reactor.oneclick() {
        println!("failed to check updates: {}", e);
    }

    println!("hello from 1.0.4");
}
