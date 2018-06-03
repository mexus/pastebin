onmessage = function(event) {
    importScripts('//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/highlight.min.js');
    importScripts('//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/languages/rust.min.js');
    if (event.data.language) {
        var result = self.hljs.highlight(event.data.language, event.data.text);
    } else {
        var result = self.hljs.highlightAuto(event.data.text);
    }
    postMessage({value: result.value, language: result.language});
}
