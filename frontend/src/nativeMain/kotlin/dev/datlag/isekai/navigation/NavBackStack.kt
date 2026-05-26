package dev.datlag.isekai.navigation

import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.runtime.snapshots.StateObject
import kotlin.reflect.KClass

/**
 * Marker interface for all screen routes.
 */
interface NavKey

/**
 * A pure Compose navigation backstack utilizing SnapshotStateList.
 */
class NavBackStack<T : NavKey>(
    private val base: SnapshotStateList<T>
) : StateObject by base, MutableList<T> by base, RandomAccess by base {

    constructor(vararg elements: T) : this(mutableStateListOf(*elements))

    fun push(key: T): Boolean = add(element = key)

    fun replaceCurrent(key: T) = mutate {
        base[lastIndex] = key
    }

    fun replaceAll(vararg keys: T): Boolean {
        if (keys.isEmpty()) {
            return false
        }

        return mutate {
            clear()
            addAll(keys)
        }
    }

    fun pop(): Boolean {
        if (size <= 1) {
            return false
        }

        return mutate {
            removeLastOrNull()
        } != null
    }

    fun popWhile(predicate: (T) -> Boolean) = mutate {
        while (size > 1 && predicate(last())) {
            removeLastOrNull()
        }
    }

    fun popTo(index: Int, inclusive: Boolean = false) = mutate {
        if (index !in indices) {
            return@mutate
        }

        val removeFrom = if (inclusive) index else index + 1

        when {
            removeFrom >= size -> return@mutate
            removeFrom <= 0 -> replaceAll(first())
            else -> removeRange(removeFrom, size)
        }
    }

    fun popTo(type: KClass<out NavKey>, inclusive: Boolean = false) {
        val index = indexOfLast { it::class == type }
        popTo(index, inclusive)
    }

    fun popTo(key: T, inclusive: Boolean = false) {
        popTo(type = key::class, inclusive)
    }

    fun popToFirst() = popTo(0)

    private fun removeRange(from: Int, to: Int) = mutate {
        subList(from, to).clear()
    }

    private fun <R> mutate(block: () -> R) = Snapshot.withMutableSnapshot(block)
}
