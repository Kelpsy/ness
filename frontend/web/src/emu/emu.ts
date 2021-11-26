import { UiToEmu, EmuToUi } from "../message";
import type * as wasm from "../../pkg";

(async () => {
    function sendMessage(message: EmuToUi.Message, transfer?: Transferable[]) {
        postMessage(message, transfer as any);
    }

    class FpsLimiter {
        private limit_!: number | null;
        private timeout!: number;
        private expectedTimeoutTime!: number;
        private timeoutId: number | undefined;

        constructor(limit: number | null, public callback: () => void) {
            this.limit = limit;
        }

        get limit(): number | null {
            return this.limit_;
        }

        set limit(limit: number | null) {
            if (limit == this.limit_) {
                return;
            }
            this.limit_ = limit;

            clearTimeout(this.timeoutId);
            this.timeout = limit === null ? 0 : 1000 / limit;
            this.expectedTimeoutTime = this.expectedTimeoutTime
                ? this.expectedTimeoutTime + this.timeout
                : performance.now() + this.timeout;
            this.timeoutId = setTimeout(
                this.handleTimeout.bind(this),
                Math.max(0, this.expectedTimeoutTime - performance.now())
            );
        }

        handleTimeout() {
            this.callback();
            this.expectedTimeoutTime += this.timeout;
            this.timeoutId = setTimeout(
                this.handleTimeout.bind(this),
                Math.max(0, this.expectedTimeoutTime - performance.now())
            );
        }
    }

    const wasm = await import("../../pkg");
    let playing = false;
    let fpsLimiter = new FpsLimiter(60, frame);
    let emu: wasm.EmuState | undefined;

    function frame() {
        if (!playing) return;
        const buffer = emu!.run_frame();
        const metadata = emu!.frame_metadata();
        if (fpsLimiter.limit !== null) {
            fpsLimiter.limit = emu!.fps_limit;
        }
        sendMessage(
            {
                type: EmuToUi.MessageType.RenderFrame,
                buffer,
                fbWidth: metadata.fb_width,
                fbHeight: metadata.fb_height,
                viewHeight: metadata.view_height,
            },
            [buffer.buffer]
        );
    }

    self.onmessage = (e) => {
        const data = e.data as UiToEmu.Message;
        switch (data.type) {
            case UiToEmu.MessageType.Start: {
                emu = wasm.create_emu_state(
                    new Uint8Array(data.romBuffer),
                    new Uint8Array(data.cartsDB),
                    new Uint8Array(data.boardsDB)
                );
                break;
            }

            case UiToEmu.MessageType.Reset: {
                emu!.reset();
                break;
            }

            case UiToEmu.MessageType.Stop: {
                close();
                break;
            }

            case UiToEmu.MessageType.LoadSave: {
                emu!.load_save(new Uint8Array(data.buffer));
                break;
            }

            case UiToEmu.MessageType.ExportSave: {
                sendMessage({
                    type: EmuToUi.MessageType.ExportSave,
                    buffer: emu!.export_save(),
                });
                break;
            }

            case UiToEmu.MessageType.UpdateInput: {
                emu!.update_input(data.pressed, data.released);
                break;
            }

            case UiToEmu.MessageType.UpdatePlaying: {
                playing = data.value;
                break;
            }

            case UiToEmu.MessageType.UpdateLimitFramerate: {
                fpsLimiter.limit = data.value ? emu!.fps_limit : null;
                break;
            }
        }
    };

    sendMessage({
        type: EmuToUi.MessageType.Loaded,
    });
})();
