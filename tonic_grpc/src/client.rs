use std::io::stdin;

use voting::{voting_client::VotingClient, VotingRequest};

use crate::voting::voting_request;

pub mod voting {
    // tonic::include_proto!("voting");
    include!("../protos/voting.rs");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = VotingClient::connect("http://[::1]:8080").await?;
    let url = "http://helloworld.com/post1";
    loop {
        let mut vote: String = String::new();
        println!("Voting for <{}>, (d)own or (u)p: ", url);
        stdin().read_line(&mut vote).unwrap();
        let vote_res = match vote.trim().to_lowercase().chars().next().unwrap() {
            'u' => voting_request::Vote::Up,
            'd' => voting_request::Vote::Down,
            _ => break,
        };
        // here comes the service invocation
        let request = tonic::Request::new(VotingRequest {
            url: String::from(url),
            vote: vote_res.into(),
        });
        let response = client.vote(request).await?;
        println!("Got: '{}' from service", response.get_ref().confirmation);
    }
    Ok(())
}
