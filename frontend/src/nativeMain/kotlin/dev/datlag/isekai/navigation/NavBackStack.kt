package dev.datlag.isekai.navigation

import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.runtime.snapshots.SnapshotStateList

/**
 * Marker interface for all screen routes.
 */
interface NavKey

/**
 * A pure Compose navigation backstack utilizing SnapshotStateList.
 */
class NavBackStack<T : NavKey>(initialKey: T) {

    internal val _items: SnapshotStateList<T> = mutableStateListOf(initialKey)
    
    /**
     * Immutable view of the current backstack.
     */
    val items: List<T> get() = _items

    /**
     * The currently active screen.
     */
    val current: T? get() = _items.lastOrNull()

    fun push(key: T) {
        _items.add(key)
    }

    fun replaceCurrent(key: T) {
        Snapshot.withMutableSnapshot {
            if (_items.isNotEmpty()) {
                _items.removeLast()
            }
            _items.add(key)
        }
    }

    fun replaceAll(vararg keys: T) {
        Snapshot.withMutableSnapshot {
            _items.clear()
            _items.addAll(keys)
        }
    }

    fun pop(): Boolean {
        return if (_items.size > 1) {
            _items.removeLast()
            true
        } else {
            false
        }
    }

    fun popWhile(predicate: (T) -> Boolean) {
        Snapshot.withMutableSnapshot {
            while (_items.isNotEmpty() && predicate(_items.last())) {
                _items.removeLast()
            }
        }
    }

    fun popTo(index: Int, inclusive: Boolean = false) {
        Snapshot.withMutableSnapshot {
            if (index < 0 || index >= _items.size) return@withMutableSnapshot
            val targetSize = if (inclusive) index else index + 1
            while (_items.size > targetSize) {
                _items.removeLast()
            }
        }
    }

    internal inline fun <reified K : T> popTo(inclusive: Boolean = false) {
        Snapshot.withMutableSnapshot {
            val index = _items.indexOfLast { it is K }
            if (index != -1) {
                popTo(index, inclusive)
            }
        }
    }

    fun popTo(key: T, inclusive: Boolean = false) {
        Snapshot.withMutableSnapshot {
            val index = _items.indexOfLast { it == key }
            if (index != -1) {
                popTo(index, inclusive)
            }
        }
    }

    fun popToFirst() {
        Snapshot.withMutableSnapshot {
            if (_items.size > 1) {
                val first = _items.first()
                _items.clear()
                _items.add(first)
            }
        }
    }
}
