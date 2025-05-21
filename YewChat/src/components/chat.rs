use serde::{Deserialize, Serialize};
use web_sys::{HtmlInputElement, KeyboardEvent, HtmlElement};
use wasm_bindgen::JsCast;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{services::{event_bus::EventBus, websocket::WebsocketService}, User};

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    InputKeyPress(KeyboardEvent),
    ToggleEmojiPicker,
    InsertEmoji(String),
}

#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
    timestamp: Option<i64>, // Add timestamp field
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
    online: bool, // Add online status
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    _producer: Box<dyn Bridge<EventBus>>,
    username: String, // Store current username to differentiate sent/received messages
    show_emoji_picker: bool, // State for emoji picker
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("Context to be set");
        
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        log::debug!("Create function");

        if let Ok(_) = wss.tx.clone().try_send(serde_json::to_string(&message).unwrap()) {
            log::debug!("Message sent successfully!");
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
            username: username.clone(),
            show_emoji_picker: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                let msg: WebSocketMessage = serde_json::from_str(&s).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let users_from_message = msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                )
                                .into(),
                                online: true, // Assume all users are online for now
                            })
                            .collect();
                        return true;
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData = serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        
                        // Auto-scroll to bottom when new message arrives
                        // Using web_sys directly instead of gloo_utils
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Some(element) = document.get_element_by_id("message-container") {
                                    if let Ok(element) = element.dyn_into::<web_sys::HtmlElement>() {
                                        element.set_scroll_top(element.scroll_height());
                                    }
                                }
                            }
                        }
                            
                        return true;
                    }
                    _ => {
                        return false;
                    }
                }
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    let message_text = input.value();
                    if !message_text.trim().is_empty() {
                        let message = WebSocketMessage {
                            message_type: MsgTypes::Message,
                            data: Some(message_text),
                            data_array: None,
                        };
                        if let Err(e) = self.wss.tx.clone().try_send(serde_json::to_string(&message).unwrap()) {
                            log::debug!("Error sending to channel: {:?}", e);
                        }
                        input.set_value("");
                    }
                }
                false
            }
            Msg::InputKeyPress(event) => {
                if event.key() == "Enter" && !event.shift_key() {
                    event.prevent_default();
                    ctx.link().send_message(Msg::SubmitMessage);
                }
                false
            }
            Msg::ToggleEmojiPicker => {
                self.show_emoji_picker = !self.show_emoji_picker;
                true
            }
            Msg::InsertEmoji(emoji) => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let current_value = input.value();
                    input.set_value(&format!("{}{}", current_value, emoji));
                    input.focus().ok();
                }
                self.show_emoji_picker = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let on_keypress = ctx.link().callback(Msg::InputKeyPress);
        let toggle_emoji = ctx.link().callback(|_| Msg::ToggleEmojiPicker);
        
        // Group users by online status
        let online_users: Vec<_> = self.users.iter().filter(|u| u.online).collect();
        let offline_users: Vec<_> = self.users.iter().filter(|u| !u.online).collect();

        html! {
            <div class="flex w-screen h-screen bg-gray-50">
                // Sidebar with users
                <div class="flex-none w-64 h-screen bg-white shadow-md">
                    <div class="flex items-center justify-between p-4 border-b">
                        <div class="text-xl font-semibold text-gray-800">{"Users"}</div>
                        <div class="bg-green-500 text-white rounded-full w-6 h-6 flex items-center justify-center">
                            {online_users.len()}
                        </div>
                    </div>
                    
                    // Search box for users
                    <div class="p-2">
                        <input 
                            type="text" 
                            placeholder="Search users..." 
                            class="w-full p-2 text-sm bg-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-400" 
                        />
                    </div>
                    
                    // Online users
                    <div class="p-2 text-xs font-medium text-gray-500">{"ONLINE"}</div>
                    <div class="overflow-y-auto max-h-64">
                        {
                            online_users.iter().map(|u| {
                                html!{
                                    <div class="flex items-center p-3 hover:bg-gray-100 rounded-lg cursor-pointer transition-colors">
                                        <div class="relative">
                                            <img class="w-10 h-10 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                                            <div class="absolute bottom-0 right-0 w-3 h-3 bg-green-500 rounded-full border-2 border-white"></div>
                                        </div>
                                        <div class="ml-3">
                                            <div class="font-medium">{u.name.clone()}</div>
                                            <div class="text-xs text-gray-500">{"Active now"}</div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    
                    // Offline users (if any)
                    if !offline_users.is_empty() {
                        <>
                            <div class="p-2 text-xs font-medium text-gray-500">{"OFFLINE"}</div>
                            <div class="overflow-y-auto max-h-48">
                                {
                                    offline_users.iter().map(|u| {
                                        html!{
                                            <div class="flex items-center p-3 hover:bg-gray-100 rounded-lg cursor-pointer opacity-60">
                                                <div class="relative">
                                                    <img class="w-10 h-10 rounded-full grayscale" src={u.avatar.clone()} alt="avatar"/>
                                                </div>
                                                <div class="ml-3">
                                                    <div class="font-medium">{u.name.clone()}</div>
                                                    <div class="text-xs text-gray-500">{"Offline"}</div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Html>()
                                }
                            </div>
                        </>
                    }
                </div>
                
                // Main chat area
                <div class="grow h-screen flex flex-col">
                    // Chat header
                    <div class="w-full h-16 bg-white shadow-sm flex items-center px-6">
                        <div class="text-xl font-semibold">{"💬 Chat Room"}</div>
                        <div class="ml-3 text-sm text-gray-500">{format!("{} participants", self.users.len())}</div>
                    </div>
                    
                    // Messages container
                    <div id="message-container" class="w-full grow overflow-auto p-6 space-y-4">
                        {
                            self.messages.iter().map(|m| {
                                let is_self = m.from == self.username;
                                let user = self.users.iter().find(|u| u.name == m.from);
                                
                                html!{
                                    <div class={classes!(
                                        "flex", 
                                        "max-w-md",
                                        if is_self { "ml-auto flex-row-reverse" } else { "" }
                                    )}>
                                        if let Some(user) = user {
                                            <img 
                                                class="w-8 h-8 rounded-full mt-1" 
                                                src={user.avatar.clone()} 
                                                alt="avatar"
                                            />
                                        }
                                        
                                        <div class={classes!(
                                            "mx-3", 
                                            "p-3", 
                                            "rounded-lg", 
                                            if is_self { 
                                                "bg-blue-500 text-white rounded-br-none" 
                                            } else { 
                                                "bg-gray-100 text-gray-800 rounded-bl-none" 
                                            }
                                        )}>
                                            if !is_self {
                                                <div class="text-sm font-medium mb-1">
                                                    {m.from.clone()}
                                                </div>
                                            }
                                            
                                            <div class={if is_self { "text-white" } else { "text-gray-800" }}>
                                                if m.message.ends_with(".gif") {
                                                    <div class="mt-1 relative">
                                                        <div class="absolute inset-0 flex items-center justify-center bg-gray-200 bg-opacity-50">
                                                            {"Loading GIF..."}
                                                        </div>
                                                        <img 
                                                            class="max-w-xs rounded" 
                                                            src={m.message.clone()} 
                                                            alt="GIF" 
                                                            onload={Callback::from(|_| {
                                                                // Handle image load event
                                                            })}
                                                        />
                                                    </div>
                                                } else {
                                                    {m.message.clone()}
                                                }
                                            </div>
                                            
                                            // Time stamp
                                            <div class={classes!(
                                                "text-xs", 
                                                "mt-1",
                                                if is_self { "text-blue-100" } else { "text-gray-500" }
                                            )}>
                                                {
                                                    m.timestamp.map_or_else(
                                                        || "Just now".to_string(),
                                                        |ts| format!("{}", ts) // Format timestamp properly in production
                                                    )
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    
                    // Input area
                    <div class="w-full bg-white p-4 shadow-lg">
                        <div class="flex items-center">
                            // Emoji picker button
                            <button 
                                onclick={toggle_emoji}
                                class="p-2 text-gray-500 hover:text-gray-700 focus:outline-none"
                            >
                                {"😀"}
                            </button>
                            
                            // Message input
                            <input 
                                ref={self.chat_input.clone()} 
                                type="text" 
                                placeholder="Type a message..." 
                                class="block w-full py-3 px-4 mx-3 bg-gray-100 rounded-full outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white" 
                                name="message" 
                                required=true 
                                onkeypress={on_keypress}
                            />
                            
                            // Send button
                            <button 
                                onclick={submit} 
                                class="p-3 bg-blue-600 hover:bg-blue-700 w-12 h-12 rounded-full flex justify-center items-center text-white transition-colors"
                            >
                                <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="w-6 h-6 fill-current">
                                    <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                                </svg>
                            </button>
                        </div>
                        
                        // Emoji picker popup (simplified version)
                        if self.show_emoji_picker {
                            <div class="absolute bottom-16 left-4 bg-white p-2 rounded-lg shadow-lg grid grid-cols-8 gap-1">
                                {
                                    ["😀", "😁", "😂", "🤣", "😃", "😄", "😅", "😆", 
                                     "😉", "😊", "😋", "😎", "😍", "😘", "🥰", "😗"].iter().map(|emoji| {
                                        let emoji_val = emoji.to_string();
                                        let on_click = ctx.link().callback(move |_| Msg::InsertEmoji(emoji_val.clone()));
                                        html! {
                                            <button 
                                                onclick={on_click} 
                                                class="w-8 h-8 hover:bg-gray-100 rounded cursor-pointer flex items-center justify-center"
                                            >
                                                {emoji}
                                            </button>
                                        }
                                    }).collect::<Html>()
                                }
                            </div>
                        }
                    </div>
                </div>
            </div>
        }
    }
}