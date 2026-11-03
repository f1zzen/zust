//! macOS: ссылки zust:// приходят Apple Event-ом GetURL, а не аргументом запуска.

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{AllocAnyThread, define_class, msg_send, sel};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSString,
};
use std::cell::RefCell;

type OnUrl = Box<dyn Fn(String)>;

thread_local! {
    static ON_URL: RefCell<Option<OnUrl>> = RefCell::new(None);
}

const GURL: u32 = u32::from_be_bytes(*b"GURL");
const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "ZustUrlHandler"]
    struct Handler;

    impl Handler {
        #[unsafe(method(handleGetURL:withReplyEvent:))]
        fn handle_get_url(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let param: Option<Retained<NSAppleEventDescriptor>> =
                unsafe { msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT] };
            let url = param.and_then(|d| d.stringValue());
            if let Some(url) = url {
                let url = url.to_string();
                ON_URL.with(|f| {
                    if let Some(f) = &*f.borrow() {
                        f(url);
                    }
                });
            }
        }

        // AppKit ставит свои обработчики при запуске - перерегистрируемся после него
        #[unsafe(method(appWillFinishLaunching:))]
        fn will_finish_launching(&self, _n: &NSNotification) {
            register(self);
        }
    }
);

fn register(handler: &Handler) {
    let manager = NSAppleEventManager::sharedAppleEventManager();
    let _: () = unsafe {
        msg_send![
            &*manager,
            setEventHandler: handler,
            andSelector: sel!(handleGetURL:withReplyEvent:),
            forEventClass: GURL,
            andEventID: GURL
        ]
    };
}

/// Вызывать в главном потоке до запуска цикла событий.
pub fn listen(on_url: impl Fn(String) + 'static) {
    ON_URL.with(|f| *f.borrow_mut() = Some(Box::new(on_url)));
    let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };
    register(&handler);
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &handler,
            sel!(appWillFinishLaunching:),
            Some(&NSString::from_str(
                "NSApplicationWillFinishLaunchingNotification",
            )),
            None,
        );
    }
    // менеджер событий не держит ссылку на обработчик - он живёт до выхода
    std::mem::forget(handler);
}
