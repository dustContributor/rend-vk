use ash::vk;

pub trait Stage {
    fn name(&self) -> &str;
    fn index(&self) -> u32;
    fn is_validation_layer_enabled(&self) -> bool;
    fn image_barriers(&self) -> Vec<vk::ImageMemoryBarrier2>;

    fn destroy(&self, device: &ash::Device);

    fn work(&mut self, ctx: super::RenderContext);

    fn wait_for_previous_frame(
        &self,
        device: &ash::Device,
        current_frame: u64,
        total_stages: u32,
        semaphore: vk::Semaphore,
    ) {
        if current_frame < 1 && self.is_validation_layer_enabled() {
            /*
             * If validation layers are enabled, don't wait the first frame to avoid
             * a validation false positive that locks the main thread for a few seconds
             */
            return;
        }
        let wait_value = [self.signal_value_for(current_frame, total_stages)];
        let pass_timeline_semaphores = [semaphore];
        let wait_info = vk::SemaphoreWaitInfo::builder()
            .values(&wait_value)
            .semaphores(&pass_timeline_semaphores)
            .build();
        unsafe {
            device
                .wait_semaphores(
                    &wait_info,
                    std::time::Duration::from_secs(1).as_nanos() as u64,
                )
                .unwrap()
        };
    }

    fn signal_next_frame(
        &self,
        device: &ash::Device,
        current_frame: u64,
        total_stages: u32,
        semaphore: vk::Semaphore,
        queue: vk::Queue,
    ) {
        let signal_value = self.signal_value_for(current_frame + 1, total_stages);
        let pass_semaphore_signal_info = [vk::SemaphoreSubmitInfo::builder()
            .semaphore(semaphore)
            .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .value(signal_value)
            .build()];
        let signal_submit_infos = [vk::SubmitInfo2::builder()
            .signal_semaphore_infos(&pass_semaphore_signal_info)
            .build()];
        unsafe {
            device
                .queue_submit2(queue, &signal_submit_infos, vk::Fence::null())
                .unwrap()
        };
    }

    fn signal_value_for(&self, current_frame: u64, total_stages: u32) -> u64 {
        crate::pipeline::signal_value_for(current_frame, total_stages, self.index())
    }
}
