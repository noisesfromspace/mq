fn main() {
    let auth_res = "mx.google.com; dkim=pass header.i=@example.com header.s=s1 header.b=abc; spf=pass (google.com: domain of user@example.com designates 1.2.3.4 as permitted sender) smtp.mailfrom=user@example.com; dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=example.com";
    
    let mut auth_summary = Vec::new();
    for part in auth_res.split(';') {
        let part = part.trim();
        if part.starts_with("dkim=") || part.starts_with("spf=") || part.starts_with("dmarc=") || part.starts_with("compauth=") {
            let value = part.split_whitespace().next().unwrap_or("");
            auth_summary.push(value.to_string());
        }
    }
    println!("Auth Summary: {}", auth_summary.join(", "));
}
