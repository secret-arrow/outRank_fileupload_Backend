use actix_web::{web, App, HttpResponse, HttpServer};
use actix_multipart::Multipart;
use futures::StreamExt;
use std::fs::{create_dir_all, File, OpenOptions};
use std::fs;
use std::path::Path;
use std::io::Write;
use actix_cors::Cors;
use serde::Serialize;

#[derive(Serialize)]
struct CanisterInfo {
    wallet_id: String,
    balance: String
}

async fn upload_file(mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_type = field.content_disposition().unwrap();
        let file_name = content_type.get_filename().unwrap();

        let file_path = format!("upload/tmp/www/{}", file_name);
        let mut file = File::create(file_path)?;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            file.write_all(&data)?;
        }

        let (status, res) = dfx_setup_and_deploy(file_name);
        if status {
            return Ok(HttpResponse::Ok().json(res));
        }
        else{
            return Ok(HttpResponse::InternalServerError().json(res));
        }
    }
    Ok(HttpResponse::Ok().json("File uploaded successfully"))
}

fn dfx_setup_and_deploy(file_name: &str) -> (bool, String) {

    let file_path = Path::new("upload/tmp/www/index.html");

    if file_path.exists() {
        let _ = fs::remove_file("upload/tmp/www/index.html");
    }

    let mut html_file = File::create("upload/tmp/www/index.html").unwrap();

    let _content = fs::read_to_string("upload/fileList.txt").unwrap();
    let url_list: Vec<&str> = _content.split('\n').collect();
    let mut temp_list: Vec<String> = Vec::new();
    for (index, link) in url_list.iter().enumerate() {
        if index == url_list.len()-1 {break;}
        let str = format!("<h4><a href='{}' target='_blank'>{}</a></h4>", link, link);
        temp_list.push(str);
    }
    let main_content = temp_list.join("\n");


    let _dfx_content = fs::read_to_string("upload/tmp/canister_ids.json").unwrap();
    let start = _dfx_content.find(r#"ic": "#).unwrap_or(0);
    let (_, aa) = _dfx_content.split_at(start+6);
    let end = aa.find(r#"""#).unwrap_or(0);
    let (canister_id, _) = aa.split_at(end);

    let index_content = format!("
        <html>
        <head>
        <title>upload</title>
        </head>
        <body>
        <h1>Welcome to outRank</h1>
        <h6></h6>
        <h3>You can find uploaded files on this Canister</h3>
        {}
        <h4><a href='https://{}.icp0.io/{}' target='_blank'>https://{}.icp0.io/{}</a></h4>
        </body>
        </html>
    ", main_content, canister_id, file_name, canister_id, file_name);

    let _ = html_file.write_all(index_content.as_bytes());


    let output = std::process::Command::new("dfx")
        .current_dir("upload/tmp")
        .arg("deploy")
        .arg("--network")
        .arg("ic")
        .output()
        .expect("failed to deploy websites");

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stderr).to_string();
        let start = result.find("www: ").unwrap_or(0);
        let (_, res) = result.split_at(start+5);
        let end = res.find("\n").unwrap_or(0);
        let (ress, _) = res.split_at(end);

        let url = format!("{}{}", ress.clone(), file_name);

        let _ = add_file_to_file_list(&url);
        return (true,url.to_string());
    }
    else{
        let result = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, result);
    }
}

fn add_file_to_file_list(content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("upload/fileList.txt")?;
    file.write_all(content.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

async fn get_file_names() -> Result<HttpResponse, actix_web::Error> {
    let content = fs::read_to_string("upload/fileList.txt")?;
    Ok(HttpResponse::Ok().json(content))
}


async fn get_canister_info() -> Result<HttpResponse, actix_web::Error> {

    let wallet_id;
    let balance;

    let output = std::process::Command::new("dfx")
    .current_dir("upload/tmp")
    .arg("identity")
    .arg("--network")
    .arg("ic")
    .arg("get-wallet")
    .output()
    .expect("failed to deploy websites");

    if output.status.success() {
        wallet_id = String::from_utf8_lossy(&output.stdout).to_string();
        let output = std::process::Command::new("dfx")
        .current_dir("upload/tmp")
        .arg("wallet")
        .arg("--network")
        .arg("ic")
        .arg("balance")
        .output()
        .expect("failed to deploy websites");

        if output.status.success() {
            balance = String::from_utf8_lossy(&output.stdout).to_string();
            let response = CanisterInfo{
                wallet_id: wallet_id,
                balance: balance
            };
            Ok(HttpResponse::Ok().json(response))
        }
        else{
            let response = CanisterInfo{
                wallet_id: wallet_id,
                balance: "Can't get balance".to_string()
            };
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
    else{
        let response = CanisterInfo{
            wallet_id: "Can't get wallet ID".to_string(),
            balance: "Can't get balance".to_string()
        };
        Ok(HttpResponse::InternalServerError().json(response))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    create_dir_all("upload/tmp/www")?;

    let mut dfx_file = File::create("upload/tmp/dfx.json").unwrap();
    let dfx_content = r#"
        {
            "canisters": {
                "www": {
                "frontend": {
                    "entrypoint": "www/index.html"
                },
                "source": ["www"],
                "type": "assets"
                }
            },
            "defaults": {
                "build": {
                    "args": "",
                    "packtool": ""
                }
            },
            "version": 1
        }
    "#;
    let _ = dfx_file.write_all(dfx_content.as_bytes());

    HttpServer::new(|| {
        let cors = Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ])
        .max_age(3600);
        App::new()
            .wrap(cors)
            .route("/upload", web::post().to(upload_file))
            .route("/all-files", web::get().to(get_file_names))
            .route("/get-info", web::get().to(get_canister_info))
    })
    .bind("127.0.0.1:7000")?
    .run()
    .await
}