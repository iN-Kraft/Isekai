package dev.datlag.isekai.common

import dev.datlag.isekai.navigation.model.DistroList

fun List<DistroList>.withConfig(remoteData: Map<String, DistroList.PublicConfig>): List<DistroList> {
    return this.map { group ->
        group.copy(groupList = group.groupList.map { distro ->
            when (distro) {
                is DistroList.Distro.Standalone -> distro.copy(
                    config = distro.config.copy(
                        available = remoteData[distro.id]?.available ?: distro.config.available,
                        version = remoteData[distro.id]?.version?.ifBlank { null } ?: distro.config.version,
                        secureBoot = remoteData[distro.id]?.secureBoot ?: distro.config.secureBoot
                    )
                )
                is DistroList.Distro.WithEditions -> distro.copy(
                    editions = distro.editions.map { edition ->
                        edition.copy(
                            config = edition.config.copy(
                                available = remoteData[edition.id]?.available ?: edition.config.available,
                                version = remoteData[edition.id]?.version?.ifBlank { null } ?: edition.config.version,
                                secureBoot = remoteData[edition.id]?.secureBoot ?: edition.config.secureBoot
                            )
                        )
                    }
                )
            }
        })
    }
}