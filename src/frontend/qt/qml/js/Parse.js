
function ListOfJsons(listJsons) {
    return listJsons.map(json => JSON.parse(json))
}

function ParseIfJson(text) {
    try {
        return JSON.parse(text)
    }
    catch(_) {
        return text
    }
}

function TryParseJson(text) {
    try {
        return JSON.parse(text)
    }
    catch(_) {
        return null
    }
}