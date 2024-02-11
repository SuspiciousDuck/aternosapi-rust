use quickjs_rs::{Context, JsValue};
use rand::prelude::*;
use scraper::Element;

#[derive(Clone)]
pub struct Server {
    pub title: String,
    pub id: String,
    pub ip: Option<String>,
}

pub struct Instance {
    client: reqwest::blocking::Client,
    cookies: String,
    session: Option<String>,
    legit_ajax: String,
    fake_ajax: (String, String, String),
    pub servers: Vec<Server>,
    pub is_logged_in: bool
}

impl Server {
    pub fn start(&self, aternos: &mut Instance) -> Result<(), String> {
        if !aternos.is_logged_in { return Err("Not logged in".to_string()); }
        let result = aternos.get_aternos("/ajax/server/start", Some(self.clone()));
        if result.is_err() { return Err(result.err().unwrap()); }
        Ok(())
    }

    pub fn stop(&self, aternos: &mut Instance) -> Result<(), String> {
        if !aternos.is_logged_in { return Err("Not logged in".to_string()); }
        let result = aternos.get_aternos("/ajax/server/stop", Some(self.clone()));
        if result.is_err() { return Err(result.err().unwrap()); }
        Ok(())
    }

    pub fn status(&self, aternos: &mut Instance) -> Result<String, String> {
        if !aternos.is_logged_in { return Err("Not logged in".to_string()); }
        let rq = aternos.get_aternos("/ajax/server/get-status", Some(self.clone()));
        if rq.is_err() { return Err(rq.err().unwrap()); }
        let result = rq.unwrap();
        let data = result.get("data").unwrap();
        Ok(data.get("label").unwrap().to_string())
    }

    pub fn players(&self, aternos: &mut Instance) -> Result<(i64, Vec<serde_json::Value>), String> {
        if !aternos.is_logged_in { return Err("Not logged in".to_string()); }
        let rq = aternos.get_aternos("/ajax/server/get-status", Some(self.clone()));
        if rq.is_err() { return Err(rq.err().unwrap()); }
        let result = rq.unwrap();
        let data = result.get("data").unwrap();
        let players = data.get("playerlist").unwrap().as_array().unwrap();
        Ok((data.get("players").unwrap().as_i64().unwrap(), players.clone()))
    }

    pub fn info(&mut self, aternos: &mut Instance) -> Result<(), String> {
        if !aternos.is_logged_in { return Err("Not logged in".to_string()); }
        let rq = aternos.get_aternos("/ajax/server/get-status", Some(self.clone()));
        if rq.is_err() { return Err(rq.err().unwrap()); }
        let result = rq.unwrap();
        let data = result.get("data").unwrap();
        self.ip = Some(format!("{}:{}", data.get("displayAddress").unwrap().as_str().unwrap(), data.get("port").unwrap().as_str().unwrap()));
        Ok(())
    }
}

impl Instance {
    pub fn new() -> Instance {
        let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:122.0) Gecko/20100101 Firefox/122.0")
        .use_rustls_tls()
        .build().unwrap();
        let legit_ajax = Self::get_legitajax(&client);
        let (key, value) = Self::get_fakeajax();
        let fake_ajax = format!("{}:{}", key, value);
        Instance { client: (client), cookies: (String::new()), session: (None),
            legit_ajax: (legit_ajax), fake_ajax: (key, value, fake_ajax),
            is_logged_in: (false), servers: Vec::new() }
    }

    pub fn find_server(&mut self, server: String) -> Result<Server, ()> {
        for s in &self.servers {
            if s.title == server { return Ok(s.clone()); }
        }
        self.fetch_servers();
        for s in &self.servers {
            if s.title == server { return Ok(s.clone()); }
        }
        Err(())
    }

    fn replace_match(regex: String, text: &String, replace: String) {
        let regex = fancy_regex::Regex::new(&regex).unwrap();
        regex.replace_all(&text, replace);
    }
    
    fn base36_encode(value: f64) -> String {
        let characters = "0123456789abcdefghijklmnopqrstuvwxyz";
        let mut result = String::new();
        let mut int_value = (value *  1e17) as u64;
        while int_value >  0 {
            result.push(characters.chars().nth((int_value %  36) as usize).unwrap());
            int_value /=  36;
        }
        while result.len() < 16 {
            result.push('0');
        }
        return result;
    }
    
    fn absorb_cookies(&mut self, rq_cookies: &mut dyn Iterator<Item = reqwest::cookie::Cookie>) {
        for cookie in rq_cookies {
            let full_cookie = format!("{}={};", cookie.name(), cookie.value());
            if !self.cookies.contains(cookie.name()) {
                self.cookies.push_str(&full_cookie);
            } else {
                let re = format!("({}=.*?;)", cookie.name());
                let replace = if cookie.value() == "deleted" { "" }
                    else { full_cookie.as_str() }.to_string();
                Self::replace_match(re, &self.cookies, replace);
            }
            if cookie.name() == "ATERNOS_SESSION" {
                self.session = Some(cookie.value().to_string());
            }
        }
    }
    
    fn eval_js(ajax: &str) -> JsValue {
        let ctx = Context::new().unwrap();
        return ctx.eval(ajax).unwrap();
    }
    
    fn get_legitajax(client: &reqwest::blocking::Client) -> String {
        let res = client
            .get("https://aternos.org/go")
            .send()
            .unwrap();
        let body = res.text().unwrap();
        let ajax_regex = fancy_regex::Regex::new(r"(AJAX_TOKEN.*\?)(.*?)(\:)").unwrap();
        let pre_ajax = ajax_regex.captures(&body).unwrap().unwrap().get(2).unwrap().as_str();
        return Self::eval_js(pre_ajax).as_str().unwrap().to_string();
    }
    
    fn get_fakeajax() -> (String, String) {
        let mut rng = thread_rng();
        return (Self::base36_encode(rng.gen()), Self::base36_encode(rng.gen()));
    }
    
    fn build_url(&self, url: &str) -> String {
        return format!("https://aternos.org{}?TOKEN={}&SEC={}", url, self.legit_ajax, self.fake_ajax.2);
    }
    
    fn generate_cookies(&mut self, server: Option<Server>) {
        let mut result= format!("ATERNOS_SEC_{}={};", self.fake_ajax.0, self.fake_ajax.1);
        if self.session.is_some() { result = format!("{} ATERNOS_SESSION={};", result, self.session.as_ref().unwrap()); }
        if server.is_some() { result = format!("{} ATERNOS_SERVER={};", result, server.unwrap().id); }
        self.cookies = result;
    }
    
    pub fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        self.generate_cookies(None);
        let login_url = Self::build_url(&self, "/ajax/account/login");
        let user = username.to_lowercase();
        let pass = md5::compute(password);
        let form_map = reqwest::blocking::multipart::Form::new()
            .text("username", user)
            .text("password", format!("{:x}", pass));
        let mut rq = self.client.post(login_url).multipart(form_map).header("Cookie", self.cookies.as_str()).send().unwrap();
        let mut binding: Vec<u8> = vec![];
        let _ = rq.copy_to(&mut binding);
        let json: serde_json::Value = serde_json::from_slice(&binding).unwrap();
        let mut cookies = rq.cookies();
        let result = json.get("success").unwrap().as_bool().unwrap() == true;
        if result {
            self.absorb_cookies(cookies.by_ref());
            self.is_logged_in = true;
            Ok(())
        } else {
            let err = json.get("error").unwrap().to_string();
            Err(err)
        }
    }
    
    pub fn fetch_servers(&mut self) {
        let html = self.client.get("https://aternos.org/servers").header("Cookie", self.cookies.clone()).send().unwrap().text().unwrap();
        let xpath_expr = ".servercard";
        let dom = scraper::Html::parse_document(&html);
        let sel = scraper::Selector::parse(xpath_expr).unwrap();
        for element in dom.select(&sel) {
            let title = element.attr("title").unwrap();
            let id = element.first_element_child().unwrap().attr("data-id").unwrap();
            let server = Server{title: title.to_string(), id: id.to_string(), ip: None};
            let mut exists = false;
            for s in &self.servers {
                if s.id == server.id { exists = true; }
            }
            if !exists { self.servers.push(server); }
        }
    }
    pub fn get_aternos(&mut self, uri: &str, server: Option<Server>) -> Result<serde_json::Map<String, serde_json::Value>, String> {
        self.generate_cookies(server);
        let url = self.build_url(uri);
        let rq = self.client.get(url).header("Cookie", &self.cookies).send().unwrap();
        let result: serde_json::Map<String, serde_json::Value> = rq.json().unwrap();
        if !result.get("success").unwrap().as_bool().unwrap() {
            return Err(result.get("error").unwrap().as_str().unwrap().to_string());
        }
        Ok(result)
    }
}