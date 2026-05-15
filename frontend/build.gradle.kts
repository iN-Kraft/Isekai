plugins {
    alias(libs.plugins.multiplatform)
    alias(libs.plugins.compose)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.serialization)
}

val fedoraSysroot = "/opt/aarch64-sysroot"
fun hasUbuntuCrossCompiler(): Boolean {
    return File("/usr/bin/aarch64-linux-gnu-pkg-config").exists()
}
fun hasFedoraSysroot(): Boolean {
    return File(fedoraSysroot).exists()
}
fun getPkgConfigCmd() = if (hasUbuntuCrossCompiler()) {
    "aarch64-linux-gnu-pkg-config"
} else {
    "pkg-config"
}
fun getArm64Sysroot(): String? = System.getenv("ARM64_SYSROOT") ?: when {
    hasUbuntuCrossCompiler() -> null
    hasFedoraSysroot() -> fedoraSysroot
    else -> null
}

fun getGlibLibs(pkgConfigCmd: String, sysroot: String?) = providers.exec {
    if (sysroot != null) {
        environment("PKG_CONFIG_SYSROOT_DIR", sysroot)
        environment("PKG_CONFIG_LIBDIR", "$sysroot/usr/lib64/pkgconfig:$sysroot/usr/share/pkgconfig")
    }
    commandLine(pkgConfigCmd, "--variable=libdir", "glib-2.0", "cairo", "gtk4", "gio-2.0", "gmodule-2.0", "graphene-1.0", "gtk4", "harfbuzz", "pango", "pangocairo", "libadwaita-1")
}.standardOutput.asText.map {
    listOf("-L${it.trim()}", "-lglib-2.0", "-lgobject-2.0", "-lgmodule-2.0", "-lgio-2.0", "-lgdk_pixbuf-2.0", "-lharfbuzz",
        "-lcairo", "-lpango-1.0", "-lpangocairo-1.0", "-lgtk-4", "-lgraphene-1.0", "-ladwaita-1", "-ldl", "-Wl,--allow-shlib-undefined")
}

kotlin {
    linuxX64 {
        binaries {
            executable {
                entryPoint = "dev.datlag.isekai.main"
                linkerOpts(getGlibLibs("pkg-config", null).get())
            }
        }
    }
    linuxArm64()
    mingwX64()

    applyDefaultHierarchyTemplate()

    sourceSets {
        commonMain.dependencies {
            implementation(libs.adwaita.compose)
            implementation(libs.coroutines)
            implementation(libs.ktor)
        }
    }
}