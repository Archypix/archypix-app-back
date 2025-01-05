use lazy_static::lazy_static;
use std::env;

use crate::utils::utils::get_frontend_host;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use rocket::serde::json::from_str;
use tera::{Context, Tera};
use tokio::task;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("src/mailing/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html"]);
        tera
    };
}

/// Sends an HTML email with the given template and context
pub fn send_rendered_email(to: (String, String), subject: String, template: String, context: Context) {
    let text = render_email_context(format!("text_{}", template), context.clone());
    let html = render_email_context(template, context);
    send_email(to, subject, text, html);
}
/// Renders an email template with the given context
/// Inserts the frontend url in the context
fn render_email_context(template: String, mut context: Context) -> String {
    context.insert("archypix_url", &get_frontend_host());
    TEMPLATES.render(format!("{}.html", template).as_str(), &context)
            .expect("Unable to render email template.")
}

/// Sends an email with the provided raw text and HTML content
fn send_email(to: (String, String), subject: String, body_text: String, body_html: String) {
    task::spawn(send_email_async(to, subject, body_text, body_html));
}

/// Sends an email with the provided raw text and HTML content asynchronously
async fn send_email_async(to: (String, String), subject: String, body_text: String, body_html: String) {
    let server: String = env::var("SMTP_SERVER").expect("SMTP_SERVER must be set");
    let server_port: u16 = env::var("SMTP_SERVER_PORT")
        .map(|port| from_str::<u16>(port.as_str()).unwrap_or(465)).unwrap_or(465);
    let from_name: String = env::var("SMTP_FROM_NAME").expect("SMTP_FROM_NAME must be set");
    let username: String = env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
    let password: String = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");

    let message = MessageBuilder::new()
        .from((from_name, username.clone()))
        .to(vec![to.clone()])
        .subject(subject)
        .text_body(body_text)
        .html_body(body_html);

    let connect = SmtpClientBuilder::new(server, server_port)
        .implicit_tls(false)
        .credentials((username, password))
        .connect()
        .await;

    if connect.is_err() {
        eprintln!("Failed to connect to SMTP server: {:?}", connect.err().unwrap());
    }else{
        if let Err(e) = connect.unwrap().send(message).await {
            eprintln!("Failed to send email: {:?}", e);
        }else{
            println!("Email sent successfully to: {} [{}]", to.0, to.1);
        }
    }
}
