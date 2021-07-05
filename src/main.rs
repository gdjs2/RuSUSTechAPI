use rocket::response::status::Unauthorized;

#[derive(serde::Serialize, serde::Deserialize)]
struct BasicInfo {
    id: String,
    sid: String,
    name: String,
    email: String,
    year: String,
    department: String,
    major: String
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SemesterGPA {
    semester_full_name: String,
    semester_year: String,
    semester_number: String,
    gpa: f64
}


#[rocket::get("/")]
async fn index() -> String {
    String::from("Hello, rustech!\n")
}

async fn login(username: &str, password: &str, client_option: Option<reqwest::Client>) -> Result<reqwest::Client, Unauthorized<String>> {
    let client = client_option.unwrap_or(reqwest::Client::builder().cookie_store(true).build().map_err(|_| Unauthorized(Some("Unable to build the client".to_owned())))?);
    let login_url = "https://cas.sustech.edu.cn/cas/login";
    let mut execution = "".to_owned();
    // Get the execution
    {
        let cas_html = client.get(login_url)
                        .send()
                        .await.map_err(|_| Unauthorized(Some(String::from("Unable to send request to cas page"))))?
                        .text()
                        .await.map_err(|_| Unauthorized(Some(String::from("Unable to the response to HTML text"))))?;
        let cas_fragment = scraper::Html::parse_fragment(&cas_html[..]);
        let input_selector = scraper::Selector::parse("input").map_err(|_| Unauthorized(Some(String::from("Unable to parse HTML to fragment"))))?;
                    
        let inputs = cas_fragment.select(&input_selector);
        for input in inputs {
            if input.value().attr("name").unwrap_or_default() == "execution" {
                execution = input.value().attr("value").unwrap_or_default().to_owned();
            }
        }
    }

    {
        // let login_post_body = format!("username={}&password={}&execution={}&_eventId=submit", username, password, execution);
        let login_post_form = [
                                ("username", username), 
                                ("password", password),
                                ("execution", &execution),
                                ("_eventId", "submit"),
                                ("locale", "en")];
        let login_resp = client.post(login_url).form(&login_post_form).send()
                                .await.map_err(|_| Unauthorized(Some(String::from("Unable to send request to cas page"))))?;

        let login_html = login_resp.text()
                                    .await.map_err(|_| Unauthorized(Some("Unable to convert the response to HTML text".to_owned())))?;
        if login_html.contains("Log In Successful") {
            println!("{:?}", client);
            Ok(client)
        } else {
            Err(Unauthorized(Some("Login Failed".to_owned())))
        }
        
        
    }
}

#[rocket::get("/cas_login?<username>&<password>")]
async fn cas_login(username: &str, password: &str) -> Result<String, Unauthorized<String>> {
    let client = login(username, password, None).await?;
    println!("{:?}", client);
    Ok("Hello world!".to_owned())
}

// #[rocket::get("/tis_login?<username>&<password>")]
async fn tis_login(username: &str, password: &str) -> Result<reqwest::Client, Unauthorized<String>> {
    let mut client = reqwest::Client::builder().cookie_store(true).gzip(true).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0").build().map_err(|_| Unauthorized(Some("Unable to build the client".to_owned())))?;
    let tis_cas_url = "https://cas.sustech.edu.cn/cas/login?service=https://tis.sustech.edu.cn/cas";
    client = login(username, password, Some(client)).await.map_err(|_| Unauthorized(Some("Unable to login".to_owned())))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Referer", reqwest::header::HeaderValue::from_static("https://tis.sustech.edu.cn/"));
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"));
    headers.insert("Accept-Language", reqwest::header::HeaderValue::from_static("zh-cn"));
    headers.insert("Accept-Encoding", reqwest::header::HeaderValue::from_static("gzip, deflate, br"));

    client.get(tis_cas_url).headers(headers).send()
            .await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?;
    // client.get("https://tis.sustech.edu.cn/user/basic").send().await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?;

    Ok(client)
}

#[rocket::get("/basic_info?<username>&<password>")]
async fn basic_info(username: &str, password: &str) -> Result<String, Unauthorized<String>> {

    let client = tis_login(username, password).await?;
    const BASIC_INFO_URL: &str = "https://tis.sustech.edu.cn/UserManager/queryxsxx";
    let v: serde_json::Value = client.post(BASIC_INFO_URL).send()
                                                        .await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?
                                                        .json::<serde_json::Value>()
                                                        .await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?;
    
    let basic_info = BasicInfo {
        id: v["ID"].as_str().unwrap_or_default().to_owned(),
        sid: v["XH"].as_str().unwrap_or_default().to_owned(),
        name: v["XM"].as_str().unwrap_or_default().to_owned(),
        email: v["DZYX"].as_str().unwrap_or_default().to_owned(),
        year: v["NJMC"].as_str().unwrap_or_default().to_owned(),
        department: v["YXMC"].as_str().unwrap_or_default().to_owned(),
        major: v["ZYMC"].as_str().unwrap_or_default().to_owned()
    };

    Ok(serde_json::to_string_pretty(&basic_info).map_err(|_| Unauthorized(Some("Unable to parse the result to JSON".to_owned())))?)
}

// TODO Null and 0.0 is different, should use Option to different them
#[rocket::get("/semester_gpa?<username>&<password>")]
async fn semester_gpa(username: &str, password: &str) -> Result<String, Unauthorized<String>> {

    let client = tis_login(username, password).await?;
    const SEMESTER_GPA_URL: &str = "https://tis.sustech.edu.cn/cjgl/xscjgl/xsgrcjcx/queryXnAndXqXfj";
    let v: serde_json::Value = client.post(SEMESTER_GPA_URL).send()
                                                        .await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?
                                                        .json::<serde_json::Value>()
                                                        .await.map_err(|_| Unauthorized(Some("Unable to send the login redirect request to CAS".to_owned())))?;
    
    let gpa_json_array = v["xnanxqxfj"].as_array().unwrap();
    let mut gpa_array = Vec::<SemesterGPA>::new();
    for gpa in gpa_json_array {
        let semester_gpa = SemesterGPA {
            semester_full_name: gpa["XNXQ"].as_str().unwrap_or_default().to_owned(),
            semester_year: gpa["XN"].as_str().unwrap_or_default().to_owned(),
            semester_number: gpa["XQ"].as_str().unwrap_or_default().to_owned(),
            gpa: gpa["XQXFJ"].as_f64().unwrap_or_default()
        };
        gpa_array.push(semester_gpa);
    }
    
    Ok(serde_json::to_string_pretty(&gpa_array).map_err(|_| Unauthorized(Some("Unable to parse the result to JSON".to_owned())))?)
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build().mount("/", rocket::routes!(index))
                    .mount("/", rocket::routes!(cas_login))
                    // .mount("/", rocket::routes!(tis_login))
                    .mount("/", rocket::routes!(basic_info))
                    .mount("/", rocket::routes!(semester_gpa))
}
