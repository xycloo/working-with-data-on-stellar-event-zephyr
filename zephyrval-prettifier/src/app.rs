use serde::{Deserialize, Serialize};
use ta::indicators::ExponentialMovingAverage;
//use log::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use base64::prelude::*;

use hex;
//use stellar_strkey::*;


#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ZephyrVal {
    I128(i128),
    I64(i64),
    U64(u64),
    F64(f64),
    U32(u32),
    I32(i32),
    F32(f32),
    String(String),
    Bytes(Vec<u8>)
}


#[derive(Default)]
pub struct App {
    hash: String,
    strkey: String,
}

pub enum AppMsg {
    Hash(String),
    Encoded(String),
}

fn decode_hex(f: impl Fn(String), val: String) {
    let bytes = if let Ok(bytes) = hex::decode(&val) {
        bytes
    } else {
        BASE64_STANDARD.decode(val).unwrap()
    };

    let decoded = if let Ok(avg) = bincode::deserialize::<ExponentialMovingAverage>(&bytes) {
        format!("{:?}", avg)
    } else {
        format!("{:?}", bincode::deserialize::<ZephyrVal>(&bytes).unwrap())
    };
    
    f(decoded);
}

trait Extend {
    fn read_id(&self) -> &str;
}

impl Extend for App {
    fn read_id(&self) -> &str {
        &self.hash
    }
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        wasm_logger::init(wasm_logger::Config::default());

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::Encoded(encoded) => {
                self.strkey = encoded;
                true
            }
            AppMsg::Hash(hash) => {
                let link = ctx.link().clone();
                decode_hex(
                    move |encoded| link.send_message(AppMsg::Encoded(encoded)),
                    hash,
                );

                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();

        let oninput = Callback::from(move |e: InputEvent| {
            let target = e.target().unwrap();
            let input = target.unchecked_into::<HtmlInputElement>();
            link.send_message(AppMsg::Hash(input.value()))
        });

        html! {
                <main>
        <div id="heading">
                    <h1>{ "Convert Stellar Event 12/02 Base64 data." }</h1>
                    <p>{ "Convert the base64 data aggregated in the custom index built on 12/02"} </p>
                    <div>
        <input oninput={oninput} />

        </div>
        </div>

        <pre><code class="language-json"> {&self.strkey
        } </code></pre>
        </main>

            }
    }
}