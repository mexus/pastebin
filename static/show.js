function highlight(type_input, language) {
    var code_container = $('pre');
    var worker = new Worker('../hljs_worker.js');
    type_input.prop('disabled', true);
    var placeholder = 'Highlighting the paste';
    if (language) {
        placeholder += ' for [' + language + ']';
    }
    placeholder += '...';
    type_input.val(placeholder);
    worker.onmessage = function(event) {
        code_container.html(event.data.value);
        type_input.val(event.data.language);
        type_input.prop('disabled', false);
        $('#contents_type_label').text(event.data.language);
    }
    var message = {text: code_container.text()};
    if (language) {
        message.language = language;
    }
    worker.postMessage(message);
}

function populate_languages() {
    var supported_languages = $('#supported_languages');
    hljs.listLanguages().forEach(function(lang){
        $(supported_languages).append(
            $('<option>', {value: lang, text: lang})
        );
    });
}

$(document).ready(function(){
    populate_languages();

    // Start a highlighting task.
    var contents_type = $('#contents_type');
    highlight(contents_type);

    // Make language selector react to 'enter' key.
    $(contents_type).keyup(function(event) {
        if (event.keyCode == 13) {
            highlight(contents_type, $(contents_type).val());
            return false;
        }
    });
})
