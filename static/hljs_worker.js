onmessage = function(event) {
    importScripts('//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/highlight.min.js');
    importScripts('//cdnjs.cloudflare.com/ajax/libs/highlight.js/9.12.0/languages/rust.min.js');
    var result = self.hljs.highlightAuto(event.data);
    postMessage({value: result.value, language: result.language});
}
