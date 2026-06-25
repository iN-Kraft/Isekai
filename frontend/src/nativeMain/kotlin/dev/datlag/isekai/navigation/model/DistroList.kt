package dev.datlag.isekai.navigation.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class DistroList(
    val groupName: String,
    val groupList: List<Distro>
) {

    @Serializable
    data class PublicConfig(
        val available: Boolean = false,
        val version: String = "",
        @SerialName("secure_boot") val secureBoot: Boolean = false
    )

    @Serializable
    data class Variant(
        val name: String,
        val id: String,
        val config: PublicConfig = PublicConfig()
    )

    @Serializable
    sealed interface Distro {
        val name: String
        val tagline: String

        @Serializable
        data class Standalone(
            override val name: String,
            override val tagline: String,
            val id: String,
            val config: PublicConfig = PublicConfig(),
            val variants: List<Variant> = emptyList()
        ) : Distro

        @Serializable
        data class WithEditions(
            override val name: String,
            override val tagline: String,
            val editions: List<Edition>
        ) : Distro {

            @Serializable
            data class Edition(
                val name: String,
                val description: String,
                val id: String,
                val config: PublicConfig = PublicConfig(),
                val variants: List<Variant> = emptyList()
            )
        }
    }

    companion object {
        val desktop by lazy {
            listOf(
                DistroList(
                    groupName = "Fedora Base",
                    groupList = listOf(
                        Distro.WithEditions(
                            name = "Fedora",
                            tagline = "It's your Operating System.",
                            editions = listOf(
                                Distro.WithEditions.Edition(
                                    name = "GNOME",
                                    description = "The leading Linux desktop",
                                    id = "fedora-gnome"
                                ),
                                Distro.WithEditions.Edition(
                                    name = "KDE",
                                    description = "The next generation personal desktop",
                                    id = "fedora-kde"
                                )
                            )
                        )
                    )
                ),
                DistroList(
                    groupName = "Ubuntu Base",
                    groupList = listOf(
                        Distro.WithEditions(
                            name = "Linux Mint",
                            tagline = "A comfortable, familiar workflow.",
                            editions = listOf(
                                Distro.WithEditions.Edition(
                                    name = "Cinnamon",
                                    description = "Sleek, modern, innovative",
                                    id = "linux-mint-cinnamon"
                                ),
                                Distro.WithEditions.Edition(
                                    name = "Xfce",
                                    description = "Light, simple, efficient",
                                    id = "linux-mint-xfce"
                                ),
                                Distro.WithEditions.Edition(
                                    name = "MATE",
                                    description = "Classic, traditional",
                                    id = "linux-mint-mate"
                                )
                            )
                        ),
                        Distro.WithEditions(
                            name = "Zorin OS",
                            tagline = "Windows Style",
                            editions = listOf(
                                Distro.WithEditions.Edition(
                                    name = "Core",
                                    description = "For basic use.",
                                    id = "zorin-os-core"
                                ),
                                Distro.WithEditions.Edition(
                                    name = "Education",
                                    description = "With educational software for schools and students.",
                                    id = "zorin-os-education"
                                )
                            )
                        )
                    )
                )
            )
        }

        val gaming by lazy {
            listOf(
                DistroList(
                    groupName = "Fedora Base",
                    groupList = listOf(
                        Distro.WithEditions(
                            name = "Bazzite",
                            tagline = "Pre-configured for Steam.",
                            editions = listOf(
                                Distro.WithEditions.Edition(
                                    name = "GNOME",
                                    description = "The leading Linux desktop",
                                    id = "bazzite-gnome"
                                ),
                                Distro.WithEditions.Edition(
                                    name = "KDE",
                                    description = "The next generation personal desktop",
                                    id = "bazzite-kde"
                                )
                            )
                        ),
                        Distro.WithEditions(
                            name = "Nobara",
                            tagline = "Fedora with gaming tweaks.",
                            editions = listOf(
                                Distro.WithEditions.Edition(
                                    name = "Official",
                                    description = "The main Nobara experience.",
                                    id = "nobara-official",
                                    variants = listOf(
                                        Variant(
                                            name = "Standard",
                                            id = "nobara-official-standard",
                                        ),
                                        Variant(
                                            name = "NVIDIA",
                                            id = "nobara-official-nvidia",
                                        )
                                    )
                                ),
                                Distro.WithEditions.Edition(
                                    name = "GNOME",
                                    description = "Clean GNOME layout with a focus on content.",
                                    id = "nobara-gnome",
                                    variants = listOf(
                                        Variant(
                                            name = "Standard",
                                            id = "nobara-gnome-standard",
                                        ),
                                        Variant(
                                            name = "NVIDIA",
                                            id = "nobara-gnome-nvidia",
                                        )
                                    )
                                ),
                                Distro.WithEditions.Edition(
                                    name = "KDE",
                                    description = "Classic KDE experience with deep customization for power users.",
                                    id = "nobara-kde",
                                    variants = listOf(
                                        Variant(
                                            name = "Standard",
                                            id = "nobara-kde-standard",
                                        ),
                                        Variant(
                                            name = "NVIDIA",
                                            id = "nobara-kde-nvidia",
                                        )
                                    )
                                )
                            )
                        )
                    )
                ),
                DistroList(
                    groupName = "Arch Base",
                    groupList = listOf(
                        Distro.Standalone(
                            name = "CachyOS",
                            tagline = "Performance-First Linux",
                            id = "cachyos"
                        )
                    )
                )
            )
        }
    }
}
