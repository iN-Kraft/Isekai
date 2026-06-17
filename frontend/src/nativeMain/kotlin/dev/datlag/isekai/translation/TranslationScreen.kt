package dev.datlag.isekai.translation

interface TranslationScreen {

    val screenIdentifier: String
    fun tr(key: String, fallback: String) = Translator.translate("${screenIdentifier}_$key", fallback)

}