// Ion's fingerprinting protection, in the page's own world before any page
// script. Like Brave's "farbling": what fingerprinting scripts read back from
// canvas, WebGL and audio gets a sliver of noise that is stable for a site
// during a session (so pages keep working) but differs between sites and
// sessions (so the readings can't follow you around). Hardware details that
// narrow you down get common values. Wrappers are Proxies, so they still look
// like native functions to `toString`.
(() => {
    const SEED = __ION_SEED__;
    const OFF = __ION_OFF_SITES__;
    const host = location.hostname;
    for (let h = host; h; h = h.includes(".") ? h.slice(h.indexOf(".") + 1) : "") {
        if (OFF.includes(h))
            return;
    }

    // A small seeded generator (mulberry32) keyed by session and site.
    let state = SEED;
    for (let i = 0; i < host.length; i++)
        state = Math.imul(state ^ host.charCodeAt(i), 2654435761) >>> 0;
    const siteSeed = state;
    function generator(extra) {
        let a = (siteSeed ^ Math.imul(extra | 0, 0x9e3779b1)) >>> 0;
        return () => {
            a = (a + 0x6d2b79f5) >>> 0;
            let t = a;
            t = Math.imul(t ^ (t >>> 15), t | 1);
            t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
            return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
        };
    }

    function wrap(object, name, make) {
        const original = object && object[name];
        if (typeof original !== "function")
            return;
        const replacement = new Proxy(original, { apply: make(original) });
        Object.defineProperty(object, name, { value: replacement, writable: true, configurable: true });
    }

    // Canvas: flip the low bit of a few pixels' colors, in the same places
    // for the same size of canvas on a site.
    function farble(data, width, height) {
        const random = generator(width * 7919 + height);
        const pixels = data.length / 4;
        const count = Math.max(1, Math.min(64, Math.floor(pixels / 1000)));
        for (let i = 0; i < count; i++) {
            const p = Math.floor(random() * pixels) * 4 + Math.floor(random() * 3);
            data[p] ^= 1;
        }
    }
    const getImageData = CanvasRenderingContext2D.prototype.getImageData;
    wrap(CanvasRenderingContext2D.prototype, "getImageData", original => (target, self, args) => {
        const image = Reflect.apply(original, self, args);
        farble(image.data, image.width, image.height);
        return image;
    });
    // toDataURL and toBlob read a noisy copy, leaving the page's canvas as is.
    function noisyCopy(canvas) {
        if (!canvas.width || !canvas.height)
            return canvas;
        const copy = document.createElement("canvas");
        copy.width = canvas.width;
        copy.height = canvas.height;
        const context = copy.getContext("2d");
        if (!context)
            return canvas;
        context.drawImage(canvas, 0, 0);
        const image = Reflect.apply(getImageData, context, [0, 0, copy.width, copy.height]);
        farble(image.data, image.width, image.height);
        context.putImageData(image, 0, 0);
        return copy;
    }
    for (const name of ["toDataURL", "toBlob"]) {
        wrap(HTMLCanvasElement.prototype, name, original => (target, self, args) =>
            Reflect.apply(original, noisyCopy(self), args));
    }

    // WebGL: the graphics card's name says a lot; report the generic one.
    const UNMASKED_VENDOR = 0x9245, UNMASKED_RENDERER = 0x9246;
    for (const proto of [window.WebGLRenderingContext?.prototype, window.WebGL2RenderingContext?.prototype]) {
        wrap(proto, "getParameter", original => (target, self, args) => {
            if (args[0] === UNMASKED_VENDOR)
                return "Google Inc.";
            if (args[0] === UNMASKED_RENDERER)
                return "ANGLE (Generic GPU)";
            return Reflect.apply(original, self, args);
        });
        wrap(proto, "readPixels", original => (target, self, args) => {
            const result = Reflect.apply(original, self, args);
            const pixels = args[6];
            if (pixels && pixels.length >= 4)
                farble(pixels, args[2] | 0, args[3] | 0);
            return result;
        });
    }

    // Audio: a tiny, inaudible offset on rendered samples.
    const touched = new WeakSet();
    wrap(window.AudioBuffer?.prototype, "getChannelData", original => (target, self, args) => {
        const data = Reflect.apply(original, self, args);
        if (!touched.has(data)) {
            touched.add(data);
            const random = generator(data.length);
            for (let i = 0; i < data.length; i += 97)
                data[i] += (random() - 0.5) * 1e-7;
        }
        return data;
    });
    wrap(window.AnalyserNode?.prototype, "getFloatFrequencyData", original => (target, self, args) => {
        const result = Reflect.apply(original, self, args);
        const array = args[0];
        const random = generator(array.length);
        for (let i = 0; i < array.length; i += 7)
            array[i] += (random() - 0.5) * 1e-4;
        return result;
    });

    // Hardware: the values most machines report.
    const define = (proto, name, value) => {
        const descriptor = Object.getOwnPropertyDescriptor(proto, name);
        if (!descriptor || !descriptor.get)
            return;
        const getter = new Proxy(descriptor.get, { apply: () => value });
        Object.defineProperty(proto, name, { get: getter, configurable: true, enumerable: descriptor.enumerable });
    };
    define(Navigator.prototype, "hardwareConcurrency", 8);
    define(Navigator.prototype, "deviceMemory", 8);
    if (window.WorkerNavigator)
        define(WorkerNavigator.prototype, "hardwareConcurrency", 8);
    // The battery level is a cross-site identifier; Firefox and Safari
    // don't offer it either.
    delete Navigator.prototype.getBattery;
})();
