export const enum FileId {
    Rom = 1 << 0,
    Save = 1 << 1,
}

export class FileInput {
    constructor(
        protected inputElement: HTMLInputElement,
        private loadCallback: (name: string, file: ArrayBuffer) => void,
        private storageKey?: string
    ) {
        inputElement.addEventListener("change", () => {
            const file = inputElement.files ? inputElement.files[0] : null;
            if (file) {
                this.loadFromInput(file);
            }
        });
    }

    get enabled(): boolean {
        return !this.inputElement.disabled;
    }

    set enabled(enabled: boolean) {
        this.inputElement.disabled = !enabled;
    }

    load(name: string, buffer: ArrayBuffer) {
        this.loadCallback(name, buffer);
    }

    unload() {}

    loadFromInput(file: File) {
        const reader = new FileReader();
        reader.onload = () => {
            const resultBuffer = reader.result as ArrayBuffer;
            if (this.storageKey) {
                reader.onload = () => {
                    this.load(file.name, resultBuffer);
                    localStorage[this.storageKey!] =
                        file.name + "," + reader.result;
                };
                reader.readAsDataURL(file);
            } else {
                this.load(file.name, resultBuffer);
            }
        };
        reader.readAsArrayBuffer(file);
    }

    loadFromStorage() {
        if (!this.storageKey) return;

        const base64 = localStorage[this.storageKey];
        if (base64) {
            const parts = base64.split(",");
            if (!parts[2]) {
                return;
            }
            const fileContents = atob(parts[2]);
            const buffer = new Uint8Array(fileContents.length);
            for (let j = fileContents.length; j--; ) {
                buffer[j] = fileContents.charCodeAt(j);
            }
            this.load(parts[0], buffer.buffer);
        }
    }
}

export class FileInputWithIndicator extends FileInput {
    private labelElement: HTMLLabelElement;
    private fileNameElement: HTMLElement;
    private loadIndicatorUse: SVGUseElement;

    constructor(
        inputElement: HTMLInputElement,
        loadCallback: (name: string, file: ArrayBuffer) => void,
        storageKey?: string
    ) {
        super(inputElement, loadCallback, storageKey);
        this.labelElement = this.inputElement
            .nextElementSibling as HTMLLabelElement;
        this.fileNameElement = this.labelElement.getElementsByClassName(
            "file-name"
        )[0] as HTMLElement;
        this.loadIndicatorUse = this.labelElement.querySelector(
            ".load-indicator > use"
        ) as SVGUseElement;
    }

    override load(name: string, buffer: ArrayBuffer) {
        super.load(name, buffer);
        this.fileNameElement.textContent = name;
        this.loadIndicatorUse.setAttributeNS(
            "http://www.w3.org/1999/xlink",
            "xlink:href",
            "file-check.svg#icon"
        );
    }

    override unload() {
        this.loadIndicatorUse.setAttributeNS(
            "http://www.w3.org/1999/xlink",
            "xlink:href",
            "file-cross.svg#icon"
        );
    }
}

export class Files {
    private loadedFiles: number = 0;
    private fileInputs: Map<FileId, FileInput>;
    public boardsDB?: ArrayBuffer;
    public cartsDB?: ArrayBuffer;

    constructor(
        private loadFileCallback: (
            id: FileId,
            name: string,
            buffer: ArrayBuffer
        ) => void,
        dbLoadedCallback: () => void,
    ) {
        this.fileInputs = new Map([
            [
                FileId.Rom,
                new FileInputWithIndicator(
                    document.getElementById("rom-input") as HTMLInputElement,
                    (name, buffer) => {
                        this.loadedFiles |= FileId.Rom;
                        this.loadFileCallback(FileId.Rom, name, buffer);
                    }
                ),
            ],
            [
                FileId.Save,
                new FileInput(
                    document.getElementById(
                        "import-save-input"
                    ) as HTMLInputElement,
                    (name, buffer) => {
                        this.loadedFiles |= FileId.Save;
                        this.loadFileCallback(FileId.Save, name, buffer);
                    },
                    "save" // TODO: Use per-game storage keys for saves
                ),
            ],
        ]);
        for (const fileInput of this.fileInputs.values()) {
            fileInput.loadFromStorage();
        }

        Promise.all([
            fetch("/resources/db/carts.bml").then((r) => r.arrayBuffer()),
            fetch("/resources/db/boards.bml").then((r) => r.arrayBuffer()),
        ]).then(([cartsDB, boardsDB]) => {
            this.cartsDB = cartsDB;
            this.boardsDB = boardsDB;
            dbLoadedCallback();
        });
    }

    loaded(name: FileId): boolean {
        return !!(this.loadedFiles & name);
    }

    toggleEnabled(name: FileId, enabled: boolean) {
        this.fileInputs.get(name)!.enabled = enabled;
    }

    unload(name: FileId) {
        if (!(this.loadedFiles & name)) return;
        this.fileInputs.get(name)!.unload();
    }
}
