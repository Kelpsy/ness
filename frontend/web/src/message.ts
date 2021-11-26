export const enum InputBits {
    R = 1 << 4,
    L = 1 << 5,
    X = 1 << 6,
    A = 1 << 7,
    Right = 1 << 8,
    Left = 1 << 9,
    Down = 1 << 10,
    Up = 1 << 11,
    Start = 1 << 12,
    Select = 1 << 13,
    Y = 1 << 14,
    B = 1 << 15,
}

export namespace UiToEmu {
    export const enum MessageType {
        Start,
        Reset,
        Stop,
        LoadSave,
        ExportSave,
        UpdateInput,
        UpdatePlaying,
        UpdateLimitFramerate,
    }

    export interface StartMessage {
        type: MessageType.Start;
        romFilename: string;
        romBuffer: Uint8Array;
        cartsDB: ArrayBuffer;
        boardsDB: ArrayBuffer;
    }

    export interface RawMessage {
        type: MessageType.Reset | MessageType.ExportSave | MessageType.Stop;
    }

    export interface LoadSaveMessage {
        type: MessageType.LoadSave;
        buffer: Uint8Array;
    }

    export interface UpdateInputMessage {
        type: MessageType.UpdateInput;
        pressed: number;
        released: number;
    }

    export interface UpdateFlagMessage {
        type: MessageType.UpdatePlaying | MessageType.UpdateLimitFramerate;
        value: boolean;
    }

    export type Message =
        | StartMessage
        | RawMessage
        | LoadSaveMessage
        | UpdateInputMessage
        | UpdateFlagMessage;
}

export namespace EmuToUi {
    export const enum MessageType {
        Loaded,
        ExportSave,
        RenderFrame,
    }

    export interface LoadedMessage {
        type: MessageType.Loaded;
    }

    export interface ExportSaveMessage {
        type: MessageType.ExportSave;
        buffer: Uint8Array;
    }

    export interface RenderFrameMessage {
        type: MessageType.RenderFrame;
        buffer: Uint32Array;
        fbWidth: number;
        fbHeight: number;
        viewHeight: number;
    }

    export type Message =
        | LoadedMessage
        | ExportSaveMessage
        | RenderFrameMessage;
}
