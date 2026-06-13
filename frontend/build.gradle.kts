import org.jetbrains.kotlin.gradle.plugin.mpp.KotlinNativeTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinNativeLink
import org.jetbrains.kotlin.konan.target.KonanTarget

plugins {
    alias(libs.plugins.multiplatform)
    alias(libs.plugins.compose)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.serialization)
}

val fedoraSysroot = "/opt/aarch64-sysroot"
val mingwSysroot = file("/usr/x86_64-w64-mingw32/sys-root/mingw")
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

fun getGlibCFlags(pkgConfigCmd: String, sysroot: String?) = providers.exec {
    if (sysroot != null) {
        environment("PKG_CONFIG_SYSROOT_DIR", sysroot)
        environment("PKG_CONFIG_LIBDIR", "$sysroot/usr/lib64/pkgconfig:$sysroot/usr/share/pkgconfig")
    }
    commandLine(pkgConfigCmd, "--cflags", "libadwaita-1")
}.standardOutput.asText

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
    compilerOptions.freeCompilerArgs.add("-opt-in=kotlinx.cinterop.ExperimentalForeignApi")

    /*linuxX64 {
        binaries {
            executable {
                entryPoint = "dev.datlag.isekai.main"
                linkerOpts(getGlibLibs("pkg-config", null).get())
            }
        }
    }*/
    // Kodein Compose does not support linuxArm64 right now
    /*linuxArm64 {
        binaries {
            executable {
                entryPoint = "dev.datlag.isekai.main"
                linkerOpts(getGlibLibs(getPkgConfigCmd(), getArm64Sysroot()).get())
            }
        }
    }*/
    mingwX64 {
        binaries {
            executable("isekai") {
                entryPoint = "dev.datlag.isekai.main"
                val mingwLibDir = providers.exec {
                    commandLine("x86_64-w64-mingw32-pkg-config", "--variable=libdir", "libadwaita-1")
                }.standardOutput.asText.map { it.trim() }.get()

                // 2. Pass the absolute paths to the .dll.a import libraries!
                // This completely prevents the LLVM linker from discovering the host's conflicting C-Runtime.
                linkerOpts(
                    "-mwindows",
                    "$mingwLibDir/libglib-2.0.dll.a",
                    "$mingwLibDir/libgobject-2.0.dll.a",
                    "$mingwLibDir/libgmodule-2.0.dll.a",
                    "$mingwLibDir/libgio-2.0.dll.a",
                    "$mingwLibDir/libgdk_pixbuf-2.0.dll.a",
                    "$mingwLibDir/libharfbuzz.dll.a",
                    "$mingwLibDir/libcairo.dll.a",
                    "$mingwLibDir/libpango-1.0.dll.a",
                    "$mingwLibDir/libpangocairo-1.0.dll.a",
                    "$mingwLibDir/libgtk-4.dll.a",
                    "$mingwLibDir/libgraphene-1.0.dll.a",
                    "$mingwLibDir/libadwaita-1.dll.a",
                    "$mingwLibDir/libintl.dll.a"
                )
            }
        }
    }

    applyDefaultHierarchyTemplate()

    targets.withType<KotlinNativeTarget> {
        val (pkgCmd, sysroot) = when (konanTarget) {
            is KonanTarget.LINUX_ARM64 -> getPkgConfigCmd() to getArm64Sysroot()
            is KonanTarget.MINGW_X64 -> "x86_64-w64-mingw32-pkg-config" to null
            else -> "pkg-config" to null
        }

        compilations.getByName("main") {
            cinterops {
                create("intl") {
                    val flags = getGlibCFlags(pkgCmd, sysroot).get().trim().split("\\s+".toRegex()).toMutableList()

                    if (konanTarget is KonanTarget.MINGW_X64) {
                        flags.add("-I${mingwSysroot.absolutePath}/include")
                    }
                    compilerOpts(flags)
                }
            }
        }
    }

    sourceSets {
        commonMain.dependencies {
            implementation(libs.adwaita.compose)
            implementation(libs.arrow)
            implementation(libs.coroutines)
            implementation(libs.ktor)
            implementation(libs.serialization.json)
            implementation(libs.kodein.compose.runtime)
            implementation(libs.locale)
        }
        all {
            languageSettings.enableLanguageFeature("ContextParameters")
        }
    }
}

val rustProjectDir = rootProject.file("backend")
val rustReleaseExe = rustProjectDir.resolve("target/x86_64-pc-windows-gnu/release/isekai-daemon.exe")
val buildRustDaemon by tasks.registering(Exec::class) {
    group = "isekai"
    description = "Builds the Rust backend daemon in release mode for Windows"
    workingDir = rustProjectDir
    commandLine("cargo", "build", "--release", "--target", "x86_64-pc-windows-gnu")

    inputs.dir(rustProjectDir.resolve("src"))
    inputs.file(rustProjectDir.resolve("Cargo.toml"))
    outputs.file(rustReleaseExe)
}

val bundleGtkDependencies by tasks.registering(Sync::class) {
    group = "isekai"
    description = "Bundles MinGW GTK/Adwaita DLLs and required share assets for offline Windows distribution"

    val intermediateDir = layout.buildDirectory.dir("gtk-bundle")
    into(intermediateDir)

    from(mingwSysroot.resolve("bin")) {
        include("libadwaita-*.dll")
        include("libgio-2.0-*.dll")
        include("libglib-2.0-*.dll")
        include("libgobject-2.0-*.dll")
        include("libgraphene-1.0-*.dll")
        include("libgtk-4-*.dll")
        include("libpango-1.0-*.dll")
        include("libpangowin32-1.0-*.dll")

        include("libcairo-*.dll")
        include("libcairo-gobject-*.dll")
        include("libpangocairo-1.0-*.dll")
        include("libpangoft2-1.0-*.dll")
        include("libharfbuzz-*.dll")
        include("libfontconfig-*.dll")
        include("libfreetype-*.dll")
        include("libepoxy-*.dll")
        include("libpixman-*.dll")
        include("libpng16-*.dll")
        include("libjpeg-*.dll")
        include("libtiff-*.dll")
        include("zlib1.dll")

        include("libffi-*.dll")
        include("libintl-*.dll")
        include("libiconv-2.dll")
        include("libgcc_s_seh-1.dll")
        include("libstdc++-*.dll")
        include("libwinpthread-*.dll")
        include("libpcre2-*.dll")
        include("libfribidi-*.dll")

        include("libappstream-*.dll")

        include("libxmlb-*.dll")
        include("libyaml-*.dll")
        include("libcurl-*.dll")
        include("libxml2-*.dll")
        include("*yaml*.dll")
        include("*xmlb*.dll")
        include("*zstd*.dll")
        include("libgcrypt-*.dll")
        include("libgpg-error-*.dll")
        include("liblzma-*.dll")

        include("libgmodule-2.0-0.dll")
        include("libcairo-script-interpreter-2.dll")
        include("*iconv*.dll")

        include("libcrypto-3-x64.dll")
        include("libssl-3-x64.dll")
        include("libidn2-0.dll")
        include("libpsl-5.dll")
        include("libssh2-1.dll")
        include("libunistring-2.dll")

        include("*gdk_pixbuf*.dll")

        // GStreamer (GTK4's Multimedia / Video Playback Backend)
        include("*gst*.dll")
        include("*gstreamer*.dll")
        include("*orc*.dll")
        include("librsvg-*.dll")
        include("libbz2-*.dll")
        include("libexpat-*.dll")
        include("libEGL*.dll")
        include("libGLESv2*.dll")
        include("libvkd3d*.dll")
        include("*d3dcompiler*.dll")
    }

    from(mingwSysroot.resolve("share/glib-2.0/schemas")) {
        into("share/glib-2.0/schemas")
        include("*.xml")
    }

    from(mingwSysroot.resolve("share/icons")) {
        into("share/icons")
        include("Adwaita/**", "hicolor/**")
    }

    from(rootProject.file("assets/dev.datlag.Isekai.svg")) {
        into("share/icons/hicolor/scalable/apps")
    }
}

val compileTranslations by tasks.registering(CompileTranslationTask::class) {
    group = "isekai"
    description = "Compiles raw .po files into binary .mo files for gettext"

    mustRunAfter(compileSchemas)

    poDir.set(layout.projectDirectory.dir("src/nativeMain/resources/locale"))
    intermediateDir.set(layout.buildDirectory.dir("gtk-bundle"))
}

val compileSchemas by tasks.registering(CompileSchemasTask::class) {
    group = "isekai"
    description = "Compiles GLib XML schemas into gschemas.compiled"

    dependsOn(bundleGtkDependencies)

    schemasDir.set(layout.buildDirectory.dir("gtk-bundle/share/glib-2.0/schemas"))
}

tasks.withType<KotlinNativeLink>().configureEach {
    if (binary.target.name == "mingwX64") {
        dependsOn(buildRustDaemon)
        dependsOn(compileSchemas)
        dependsOn(compileTranslations)

        val sourceExe = rustReleaseExe
        val outputDirProvider = destinationDirectory
        val bundleDirProvider = bundleGtkDependencies.map { it.destinationDir }

        doLast {
            val outputDir = outputDirProvider.get().asFile
            val targetExe = outputDir.resolve(sourceExe.name)

            if (sourceExe.exists()) {
                sourceExe.copyTo(targetExe, overwrite = true)
                println("Staged Daemon: ${targetExe.name}")
            } else {
                System.err.println("WARNING: Daemon not found at ${sourceExe.absolutePath}")
            }

            val bundleDir = bundleDirProvider.get()
            if (bundleDir.exists()) {
                bundleDir.copyRecursively(outputDir, overwrite = true)
                println("Staged GTK Bundle (DLLs and Assets) into: $outputDir")
            }
        }
    }
}

abstract class CompileSchemasTask : DefaultTask() {
    @get:Inject
    abstract val execOperations: ExecOperations

    @get:InputDirectory
    abstract val schemasDir: DirectoryProperty

    @TaskAction
    fun compile() {
        val dir = schemasDir.get().asFile

        if (dir.exists()) {
            execOperations.exec {
                commandLine("glib-compile-schemas", dir.absolutePath)
            }
        }
    }
}

abstract class CompileTranslationTask : DefaultTask() {

    @get:Inject
    abstract val execOperations: ExecOperations

    @get:InputDirectory
    abstract val poDir: DirectoryProperty

    @get:OutputDirectory
    abstract val intermediateDir: DirectoryProperty

    @TaskAction
    fun compile() {
        val po = poDir.get().asFile
        val out = intermediateDir.get().asFile

        if (!po.exists()) {
            return
        }

        po.listFiles { file -> file.extension == "po" }?.forEach { poFile ->
            val lang = poFile.nameWithoutExtension
            val targetDir = out.resolve("share/locale/$lang/LC_MESSAGES")
            targetDir.mkdirs()

            val moFile = targetDir.resolve("isekai.mo")

            execOperations.exec {
                commandLine("msgfmt", "-o", moFile.absolutePath, poFile.absolutePath)
            }
        }
    }
}