rootProject.name = "Isekai"
enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

include(":frontend")

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        iNKraftRepository("Native-Kommons")
    }
}

fun findProperty(key: String): String? {
    val localProperties = java.util.Properties().apply {
        val file = rootDir.resolve("local.properties")
        if (file.exists()) {
            file.inputStream().use {
                load(it)
            }
        }
    }

    return providers.gradleProperty(key).orNull?.ifBlank {
        null
    } ?: localProperties.getProperty(key)?.ifBlank { null }
}

fun RepositoryHandler.iNKraftRepository(repository: String) {
    maven {
        name = "iNKraft $repository"
        url = uri("https://maven.pkg.github.com/iN-Kraft/$repository")
        credentials {
            username = findProperty("githubPackagesUsername") ?: System.getenv("PACKAGING_USERNAME")?.ifBlank { null }
            password = findProperty("githubPackagesPassword") ?: System.getenv("PACKAGING_PASSWORD")?.ifBlank { null }
        }
    }
}