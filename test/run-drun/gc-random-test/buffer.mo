// Source: Motoko Base Library.
// Reduced to a subset of the required functionality.

import Prim "mo:⛔";

module {
    // The following constants are used to manage the capacity.
    // The length of `elements` is increased by `INCREASE_FACTOR` when capacity is reached.
    // The length of `elements` is decreased by `DECREASE_FACTOR` when capacity is strictly less than
    // `DECREASE_THRESHOLD`.

    // INCREASE_FACTOR = INCREASE_FACTOR_NUME / INCREASE_FACTOR_DENOM (with floating point division)
    // Keep INCREASE_FACTOR low to minimize cycle limit problem
    private let INCREASE_FACTOR_NUME = 3;
    private let INCREASE_FACTOR_DENOM = 2;
    private let DECREASE_THRESHOLD = 4; // Don't decrease capacity too early to avoid thrashing
    private let DECREASE_FACTOR = 2;
    
    private func newCapacity(oldCapacity : Nat) : Nat {
        if (oldCapacity == 0) {
            1;
        } else {
            // calculates ceil(oldCapacity * INCREASE_FACTOR) without floats
            ((oldCapacity * INCREASE_FACTOR_NUME) + INCREASE_FACTOR_DENOM - 1) / INCREASE_FACTOR_DENOM;
        };
    };

    public class Buffer<X>(initCapacity : Nat) = this {
        var _size : Nat = 0; // avoid name clash with `size()` method
        var elements : [var ?X] = Prim.Array_init(initCapacity, null);

        /// Returns the current number of elements in the buffer.
        ///
        /// Example:
        /// ```motoko include=initialize
        /// buffer.size() // => 0
        /// ```
        ///
        /// Runtime: O(1)
        ///
        /// Space: O(1)
        public func size() : Nat = _size;

        /// Adds a single element to the end of the buffer, doubling
        /// the size of the array if capacity is exceeded.
        ///
        /// Example:
        /// ```motoko include=initialize
        ///
        /// buffer.add(0); // add 0 to buffer
        /// buffer.add(1);
        /// buffer.add(2);
        /// buffer.add(3); // causes underlying array to increase in capacity
        /// Buffer.toArray(buffer) // => [0, 1, 2, 3]
        /// ```
        ///
        /// Amortized Runtime: O(1), Worst Case Runtime: O(size)
        ///
        /// Amortized Space: O(1), Worst Case Space: O(size)
        public func add(element : X) {
            if (_size == elements.size()) {
                reserve(newCapacity(elements.size()));
            };
            elements[_size] := ?element;
            _size += 1;
        };

        /// Returns the element at index `index`. Traps if  `index >= size`. Indexing is zero-based.
        ///
        /// Example:
        /// ```motoko include=initialize
        ///
        /// buffer.add(10);
        /// buffer.add(11);
        /// buffer.get(0); // => 10
        /// ```
        ///
        /// Runtime: O(1)
        ///
        /// Space: O(1)
        public func get(index : Nat) : X {
            elements[index] ?? Prim.trap("Buffer index out of bounds in get");
        };

        
        /// Removes and returns the last item in the buffer or `null` if
        /// the buffer is empty.
        ///
        /// Example:
        /// ```motoko include=initialize
        ///
        /// buffer.add(10);
        /// buffer.add(11);
        /// buffer.removeLast(); // => ?11
        /// ```
        ///
        /// Amortized Runtime: O(1), Worst Case Runtime: O(size)
        ///
        /// Amortized Space: O(1), Worst Case Space: O(size)
        public func removeLast() : ?X {
            if (_size == 0) {
                return null;
            };

            _size -= 1;
            let lastElement = elements[_size];
            elements[_size] := null;

            if (_size < elements.size() / DECREASE_THRESHOLD) {
                // FIXME should this new capacity be a function of _size
                // instead of the current capacity? E.g. _size * INCREASE_FACTOR
                reserve(elements.size() / DECREASE_FACTOR);
            };

            lastElement;
        };

        /// Changes the capacity to `capacity`. Traps if `capacity` < `size`.
        ///
        /// ```motoko include=initialize
        ///
        /// buffer.reserve(4);
        /// buffer.add(10);
        /// buffer.add(11);
        /// buffer.capacity(); // => 4
        /// ```
        ///
        /// Runtime: O(capacity)
        ///
        /// Space: O(capacity)
        public func reserve(capacity : Nat) {
            if (capacity < _size) {
                Prim.trap "capacity must be >= size in reserve";
            };

            let elements2 = Prim.Array_init<?X>(capacity, null);

            var i = 0;
            while (i < _size) {
                elements2[i] := elements[i];
                i += 1;
            };
            elements := elements2;
        };

        /// Returns an Iterator (`Iter`) over the elements of this buffer.
        /// Iterator provides a single method `next()`, which returns
        /// elements in order, or `null` when out of elements to iterate over.
        ///
        /// ```motoko include=initialize
        ///
        /// buffer.add(10);
        /// buffer.add(11);
        /// buffer.add(12);
        ///
        /// var sum = 0;
        /// for (element in buffer.values()) {
        ///   sum += element;
        /// };
        /// sum // => 33
        /// ```
        ///
        /// Runtime: O(1)
        ///
        /// Space: O(1)
        public func values() : { next : () -> ?X } = object {
            // FIXME either handle modification to underlying list
            // or explicitly warn users in documentation
            var nextIndex = 0;
            public func next() : ?X {
                if (nextIndex >= _size) {
                    return null;
                };
                let nextElement = elements[nextIndex];
                nextIndex += 1;
                nextElement;
            };
        };
    };

    /// Returns true iff `buffer` contains `element` with respect to equality
    /// defined by `equal`.
    ///
    ///
    /// Example:
    /// ```motoko include=initialize
    /// import Nat "mo:base/Nat";
    ///
    /// buffer.add(2);
    /// buffer.add(0);
    /// buffer.add(3);
    /// Buffer.contains<Nat>(buffer, 2, Nat.equal); // => true
    /// ```
    ///
    /// Runtime: O(size)
    ///
    /// Space: O(1)
    ///
    /// *Runtime and space assumes that `equal` runs in O(1) time and space.
    public func contains<X>(buffer : Buffer<X>, element : X, equal : (X, X) -> Bool) : Bool {
        for (current in buffer.values()) {
            if (equal(current, element)) {
                return true;
            };
        };

        false;
    };
};
