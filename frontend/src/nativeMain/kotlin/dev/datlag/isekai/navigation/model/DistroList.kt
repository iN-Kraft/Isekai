package dev.datlag.isekai.navigation.model

import kotlinx.serialization.Serializable

@Serializable
data class DistroList(
    val groupName: String,
    val groupList: List<Distro>
) {

    @Serializable
    data class Distro(
        val name: String,
        val tagline: String,
        val editions: List<Edition>
    ) {

        @Serializable
        data class Edition(
            val name: String,
            val description: String
        )
    }

    companion object {
        val desktop by lazy {
            listOf(
                DistroList(
                    groupName = "Fedora Base",
                    groupList = listOf(
                        Distro(
                            name = "Fedora",
                            tagline = "It's your Operating System.",
                            editions = listOf(
                                Distro.Edition(
                                    name = "GNOME",
                                    description = "The leading Linux desktop"
                                ),
                                Distro.Edition(
                                    name = "KDE",
                                    description = "The next generation personal desktop"
                                )
                            )
                        )
                    )
                ),
                DistroList(
                    groupName = "Ubuntu Base",
                    groupList = listOf(
                        Distro(
                            name = "Linux Mint",
                            tagline = "A comfortable, familiar workflow.",
                            editions = listOf(
                                Distro.Edition(
                                    name = "Cinnamon",
                                    description = "Sleek, modern, innovative"
                                ),
                                Distro.Edition(
                                    name = "Xfce",
                                    description = "Light, simple, efficient"
                                ),
                                Distro.Edition(
                                    name = "MATE",
                                    description = "Classic, traditional"
                                )
                            )
                        ),
                        Distro(
                            name = "Zorin OS",
                            tagline = "Windows Style",
                            editions = listOf(
                                Distro.Edition(
                                    name = "Core",
                                    description = "For basic use."
                                ),
                                Distro.Edition(
                                    name = "Education",
                                    description = "With educational software for schools and students."
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
                        Distro(
                            name = "Bazzite",
                            tagline = "Pre-configured for Steam.",
                            editions = listOf(
                                Distro.Edition(
                                    name = "GNOME",
                                    description = "The leading Linux desktop"
                                ),
                                Distro.Edition(
                                    name = "KDE",
                                    description = "The next generation personal desktop"
                                )
                            )
                        ),
                        Distro(
                            name = "Nobara",
                            tagline = "Fedora with gaming tweaks.",
                            editions = listOf(
                                Distro.Edition(
                                    name = "Official",
                                    description = "The main Nobara experience."
                                ),
                                Distro.Edition(
                                    name = "GNOME",
                                    description = "Clean GNOME layout with a focus on content."
                                ),
                                Distro.Edition(
                                    name = "KDE",
                                    description = "Classic KDE experience with deep customization for power users."
                                )
                            )
                        )
                    )
                ),
                DistroList(
                    groupName = "Arch Base",
                    groupList = listOf(
                        Distro(
                            name = "CachyOS",
                            tagline = "Performance-First Linux",
                            editions = emptyList()
                        )
                    )
                )
            )
        }
    }
}
