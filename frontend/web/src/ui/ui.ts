import { UiToEmu, EmuToUi } from "../message";
import { FileId, Files } from "./files";
import { Input } from "./input";
import vertShaderSource from "raw-loader!../shaders/screen.vert";
import fragShaderSource from "raw-loader!../shaders/screen.frag";
import { isMobileBrowser } from "./utils";

export class Ui {
    private canvasContainer: HTMLElement;
    private canvas: HTMLCanvasElement;
    private input: Input;
    private inputState: number;
    private menuContainer: HTMLElement;
    private playButton: HTMLButtonElement;
    private resetButton: HTMLButtonElement;
    private exportSaveButton: HTMLButtonElement;

    private files: Files;

    private gl: WebGLRenderingContext;
    private fbTexture: WebGLTexture;
    private fbProgram: WebGLProgram;
    private fbCoordsAttrib: number;
    private fbMaxUVUniform: WebGLUniformLocation;
    private fbBuffer: WebGLBuffer;

    private fbWidth: number;
    private fbHeight: number;
    private fbViewHeight: number;

    private worker: Worker | undefined;

    constructor(touch: boolean) {
        this.canvasContainer = document.getElementById(
            "canvas-container"
        ) as HTMLDivElement;
        this.canvas = document.getElementById("canvas") as HTMLCanvasElement;
        this.input = new Input(touch, this.pause.bind(this));
        this.inputState = 0;
        this.menuContainer = document.getElementById(
            "menu-container"
        ) as HTMLElement;
        this.playButton = document.getElementById("play") as HTMLButtonElement;
        this.resetButton = document.getElementById(
            "reset"
        ) as HTMLButtonElement;
        this.exportSaveButton = document.getElementById(
            "export-save"
        ) as HTMLButtonElement;

        this.files = new Files(
            (id, name, buffer) => {
                switch (id) {
                    case FileId.Rom: {
                        this.files.unload(FileId.Save);
                        this.start(name, buffer);
                        break;
                    }
                    case FileId.Save: {
                        this.sendMessage(
                            {
                                type: UiToEmu.MessageType.LoadSave,
                                buffer: new Uint8Array(buffer),
                            },
                            [buffer]
                        );
                        break;
                    }
                    default:
                        break;
                }
            },
            () => this.files.toggleEnabled(FileId.Rom, true)
        );

        this.playButton.addEventListener("click", this.play.bind(this));

        this.resetButton.addEventListener("click", () => {
            this.sendMessage({
                type: UiToEmu.MessageType.Reset,
            });
        });

        this.exportSaveButton.addEventListener("click", () => {
            this.sendMessage({
                type: UiToEmu.MessageType.ExportSave,
            });
        });

        const gl = this.canvas.getContext("webgl", {
            alpha: false,
            depth: false,
            stencil: false,
            antialias: false,
            powerPreference: "low-power",
        });
        if (!gl) {
            throw new Error("Couldn't create WebGL context");
        }
        this.gl = gl;

        const fbTexture = gl.createTexture()!;
        this.fbTexture = fbTexture;
        gl.bindTexture(gl.TEXTURE_2D, fbTexture);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texImage2D(
            gl.TEXTURE_2D,
            0,
            gl.RGBA,
            512,
            478,
            0,
            gl.RGBA,
            gl.UNSIGNED_BYTE,
            new Uint8Array(512 * 478 * 4)
        );

        const vertShader = gl.createShader(gl.VERTEX_SHADER)!;
        gl.shaderSource(vertShader, vertShaderSource);
        gl.compileShader(vertShader);
        if (!gl.getShaderParameter(vertShader, gl.COMPILE_STATUS)) {
            throw new Error(
                `WebGL vertex shader compilation failed: ${gl.getShaderInfoLog(
                    vertShader
                )}`
            );
        }

        const fragShader = gl.createShader(gl.FRAGMENT_SHADER)!;
        gl.shaderSource(fragShader, fragShaderSource);
        gl.compileShader(fragShader);
        if (!gl.getShaderParameter(fragShader, gl.COMPILE_STATUS)) {
            throw new Error(
                `WebGL fragment shader compilation failed: ${gl.getShaderInfoLog(
                    fragShader
                )}`
            );
        }

        const fbProgram = gl.createProgram()!;
        gl.attachShader(fbProgram, vertShader);
        gl.attachShader(fbProgram, fragShader);
        gl.linkProgram(fbProgram);
        if (!gl.getProgramParameter(fbProgram, gl.LINK_STATUS)) {
            throw new Error(
                `WebGL program linking failed: ${gl.getProgramInfoLog(
                    fbProgram
                )}`
            );
        }
        this.fbProgram = fbProgram;

        this.fbCoordsAttrib = gl.getAttribLocation(fbProgram, "coords");

        this.fbMaxUVUniform = gl.getUniformLocation(this.fbProgram, "uv_max")!;
        const fbBuffer = gl.createBuffer()!;
        gl.bindBuffer(gl.ARRAY_BUFFER, fbBuffer);
        gl.bufferData(
            gl.ARRAY_BUFFER,
            new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]),
            gl.STATIC_DRAW
        );
        this.fbBuffer = fbBuffer;

        this.fbWidth = 256;
        this.fbHeight = 224;
        this.fbViewHeight = 224;
        this.frame();
    }

    frame() {
        const containerWidth = this.canvasContainer.clientWidth;
        const containerHeight = this.canvasContainer.clientHeight;
        const fbAspectRatio = 256 / this.fbViewHeight;
        let width = Math.floor(Math.min(containerHeight * fbAspectRatio, containerWidth));
        let height = Math.floor(width / fbAspectRatio);
        this.canvas.style.width = `${width}px`;
        this.canvas.style.height = `${height}px`;
        width = width * Math.round(window.devicePixelRatio);
        height = height * Math.round(window.devicePixelRatio);
        this.canvas.width = width;
        this.canvas.height = height;
        this.gl.viewport(0, 0, width, height);

        this.gl.clearColor(0, 0, 0, 1.0);
        this.gl.clear(this.gl.COLOR_BUFFER_BIT);
        this.gl.useProgram(this.fbProgram);
        this.gl.vertexAttribPointer(
            this.fbCoordsAttrib,
            2,
            this.gl.FLOAT,
            false,
            8,
            0
        );
        this.gl.enableVertexAttribArray(this.fbCoordsAttrib);
        this.gl.uniform2f(
            this.fbMaxUVUniform,
            this.fbWidth / 512,
            this.fbHeight / 478
        );
        this.gl.drawArrays(this.gl.TRIANGLE_STRIP, 0, 4);

        const prevInputState = this.inputState;
        this.inputState = this.input.process();
        if (this.inputState != prevInputState) {
            this.sendMessage({
                type: UiToEmu.MessageType.UpdateInput,
                pressed: this.inputState & ~prevInputState,
                released: prevInputState & ~this.inputState,
            });
        }

        requestAnimationFrame(this.frame.bind(this));
    }

    sendMessage(message: UiToEmu.Message, transfer?: Transferable[]) {
        this.worker?.postMessage(message, transfer as any);
    }

    play() {
        document.body.classList.remove("paused");
        this.sendMessage({
            type: UiToEmu.MessageType.UpdatePlaying,
            value: true,
        });
    }

    pause() {
        document.body.classList.add("paused");
        this.sendMessage({
            type: UiToEmu.MessageType.UpdatePlaying,
            value: false,
        });
    }

    stop() {
        if (!this.worker) return;
        this.files.toggleEnabled(FileId.Save, false);
        this.exportSaveButton.disabled = true;
        this.playButton.disabled = true;
        this.resetButton.disabled = true;
        this.sendMessage({
            type: UiToEmu.MessageType.Stop,
        });
        this.worker = undefined;
    }

    start(romFilename: string, romBuffer: ArrayBuffer) {
        this.stop();
        this.files.toggleEnabled(FileId.Save, true);
        this.exportSaveButton.disabled = false;
        this.playButton.disabled = false;
        this.resetButton.disabled = false;
        this.worker = new Worker("emu.bundle.js");
        this.worker.onmessage = () => {
            this.sendMessage(
                {
                    type: UiToEmu.MessageType.Start,
                    romFilename,
                    romBuffer: new Uint8Array(romBuffer),
                    cartsDB: this.files.cartsDB!,
                    boardsDB: this.files.boardsDB!,
                },
                [romBuffer]
            );
            this.worker!.onmessage = (e) => {
                this.handleWorkerEvent(e.data);
            };
        };
    }

    handleWorkerEvent(event: EmuToUi.Message) {
        switch (event.type) {
            case EmuToUi.MessageType.ExportSave: {
                // TODO
                console.error("TODO: Export save");
                break;
            }

            case EmuToUi.MessageType.RenderFrame: {
                this.gl.texSubImage2D(
                    this.gl.TEXTURE_2D,
                    0,
                    0,
                    0,
                    512,
                    event.fbHeight,
                    this.gl.RGBA,
                    this.gl.UNSIGNED_BYTE,
                    new Uint8Array(event.buffer.buffer)
                );
                this.fbWidth = event.fbWidth;
                this.fbHeight = event.fbHeight;
                this.fbViewHeight = event.viewHeight;
            }
        }
    }
}

export const ui = new Ui(isMobileBrowser());
