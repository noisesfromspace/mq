use mailparse::*;
use std::fs;

fn main() {
    let content = b"Authentication-Results: mx.google.com;
       dkim=pass header.i=@example.com header.s=s1 header.b=abc;
       spf=pass (google.com: domain of user@example.com designates 1.2.3.4 as permitted sender) smtp.mailfrom=user@example.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=example.com
Received-SPF: pass (google.com: domain of user@example.com designates 1.2.3.4 as permitted sender) client-ip=1.2.3.4;

Body here.";
    let parsed = parse_mail(content).unwrap();
    for header in parsed.get_headers() {
        println!("{}: {}", header.get_key(), header.get_value());
    }
}
