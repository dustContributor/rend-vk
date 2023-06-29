use ash::extensions::ext::DebugUtils;
use ash::vk;
use std::borrow::Cow;
use std::ffi::CStr;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let msg_id: i32 = callback_data.message_id_number as i32;
    let msg_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };
    let msg = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    log::debug!(
        "{:?}:{:?} [{} ({})]: {}",
        message_severity,
        message_type,
        msg_name,
        &msg_id.to_string(),
        msg,
    );
    vk::FALSE
}

pub struct DebugContext {
    loader: DebugUtils,
    callback: vk::DebugUtilsMessengerEXT,
}

impl DebugContext {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = DebugUtils::new(&entry, &instance);
        let debug_call_back =
            unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None) }.unwrap();
        return DebugContext {
            loader: debug_utils_loader,
            callback: debug_call_back,
        };
    }
    pub fn destroy(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_utils_messenger(self.callback, None);
        }
    }
}
