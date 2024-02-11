use quickjs_rs::{Context, JsValue};
use rand::prelude::*;
use scraper::Element;

pub struct Server {
    pub title: String,
    pub id: String
}

pub struct Instance {
    client: reqwest::blocking::Client,
    cookies: String,
    legit_ajax: String,
    fake_ajax: (String, String, String)
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
        let cookies = Self::generate_cookies(key.clone(), value.clone(), None, None);
    Instance { client: (client), cookies: (cookies), legit_ajax: (legit_ajax), fake_ajax: (key, value, fake_ajax) }
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
    
    fn build_url(url: &str, legit_ajax: String, fake_ajax: String) -> String {
        return format!("https://aternos.org{}?TOKEN={}&SEC={}", url, legit_ajax, fake_ajax);
    }
    
    fn generate_cookies(key: String, value: String, session: Option<String>, server: Option<String>) -> String {
        let mut result= format!("ATERNOS_SEC_{}={};", key, value);
        if session.is_some() { result = format!("{} ATERNOS_SESSION={};", result, session.unwrap()); }
        if server.is_some() { result = format!("{} ATERNOS_SERVER={};", result, server.unwrap()); }
        return result;
    }
    
    pub fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let login_url = Self::build_url("/ajax/account/login", self.legit_ajax.clone(), self.fake_ajax.2.clone());
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
            Self::absorb_cookies(self, cookies.by_ref());
            Ok(())
        } else {
            let err = json.get("error").unwrap().to_string();
            Err(err)
        }
    }
    
    pub fn get_servers(&self) -> Vec<Server> {
        let html = self.client.get("https://aternos.org/servers").header("Cookie", self.cookies.clone()).send().unwrap().text().unwrap();
        let xpath_expr = ".servercard";
        let dom = scraper::Html::parse_document(&html);
        let sel = scraper::Selector::parse(xpath_expr).unwrap();
        let mut list: Vec<Server> = Vec::new();
        for element in dom.select(&sel) {
            let title = element.attr("title").unwrap();
            let id = element.first_element_child().unwrap().attr("data-id").unwrap();
            let server = Server{title: title.to_string(), id: id.to_string()};
            list.push(server);
        }
        return list;
    }
}