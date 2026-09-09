import Prim "mo:prim";
import Array "array";
import Random "random";
import Values "values";
import Region "../stable-region/Region";

module {
    public type RandomType = {
        name : Text;
        allocationLimit : ?Nat;
        instantiate(random : Random.Random) : Values.RandomValue;
        readReferences(source : Values.RandomValue) : [Values.RandomValue];
        append(target : Values.RandomValue, value : Values.RandomValue) : Bool;
        randomWrite(random : Random.Random, target : Values.RandomValue, value : Values.RandomValue) : ();
        randomCheck(random : Random.Random, value : Values.RandomValue) : ();
        identical(left : Values.RandomValue, right : Values.RandomValue) : Bool;
        hash(value : Values.RandomValue) : Nat;
        text(value : Values.RandomValue) : Text;
        shortText(value : Values.RandomValue) : Text;
    };

    public func allocationTypes() : [RandomType] {
        [RandomObjectType, RandomArrayType, RandomBlobType, RandomBoxType, RandomTextType, RandomOptionType, RandomRegionType];
    };

    public func getType(value : Values.RandomValue) : RandomType {
        switch value {
            case (#none) NoneType;
            case (#obj _) RandomObjectType;
            case (#arr _) RandomArrayType;
            case (#blob _) RandomBlobType;
            case (#box _) RandomBoxType;
            case (#text _) RandomTextType;
            case (#option _) RandomOptionType;
            case (#region _) RandomRegionType;
        };
    };

    public func isIdentical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
        let leftType = getType(left);
        let rightType = getType(right);
        if (leftType.name == rightType.name) {
            leftType.identical(left, right);
        } else {
            false;
        };
    };

    public func toHash(value : Values.RandomValue) : Nat {
        getType(value).hash(value);
    };

    public func toText(value : Values.RandomValue) : Text {
        getType(value).text(value);
    };

    public func toShortText(value : Values.RandomValue) : Text {
        getType(value).shortText(value);
    };

    public object NoneType {
        public let name = "none";
        public let allocationLimit : ?Nat = null;

        public func instantiate(_random : Random.Random) : Values.RandomValue {
            Prim.trap("unsupported");
        };

        public func readReferences(_source : Values.RandomValue) : [Values.RandomValue] {
            Prim.trap("unsupported");
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {
            Prim.trap("unsupported");
        };

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            assert (Values.isNone(value));
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            assert (Values.isNone(left) and Values.isNone(right));
            true;
        };

        public func hash(_value : Values.RandomValue) : Nat {
            0;
        };

        public func text(_value : Values.RandomValue) : Text {
            name;
        };

        public func shortText(_value : Values.RandomValue) : Text {
            name;
        };
    };

    public object RandomObjectType {
        public let name = "object";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let number = random.next();
            let randomObject = {
                var randomNumber = number;
                var ref1 = #none;
                var ref2 = #none;
                checkNumber = number;
            } : Values.RandomObject;
            #obj randomObject;
        };

        public func readReferences(source : Values.RandomValue) : [Values.RandomValue] {
            let randomObject = cast(source);
            Values.filterReferences([randomObject.ref1, randomObject.ref2]);
        };

        public func append(target : Values.RandomValue, value : Values.RandomValue) : Bool {
            let randomObject = cast(target);
            if (Values.isNone(randomObject.ref1)) {
                randomObject.ref1 := value;
                true;
            } else if (Values.isNone(randomObject.ref2)) {
                randomObject.ref2 := value;
                true;
            } else {
                false;
            };
        };

        public func randomWrite(random : Random.Random, target : Values.RandomValue, value : Values.RandomValue) {
            let randomObject = cast(target);
            if (random.next() % 2 == 0) {
                randomObject.ref1 := value;
            } else {
                randomObject.ref2 := value;
            };
        };

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            let obj = cast(value);
            assert (obj.randomNumber == obj.checkNumber);
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftObject = cast(left);
            let rightObject = cast(right);
            if (leftObject.randomNumber == rightObject.randomNumber) {
                leftObject.randomNumber += 1;
                let same = leftObject.randomNumber == rightObject.randomNumber;
                leftObject.randomNumber -= 1;
                assert (leftObject.randomNumber == leftObject.checkNumber);
                assert (rightObject.randomNumber == rightObject.checkNumber);
                same;
            } else {
                false;
            };
        };

        public func hash(value : Values.RandomValue) : Nat {
            let obj = cast(value);
            obj.randomNumber;
        };

        public func text(value : Values.RandomValue) : Text {
            let obj = cast(value);
            name # "(number = " # debug_show (obj.randomNumber) # ", ref1 = " # toShortText(obj.ref1) # ", ref2 = " # toShortText(obj.ref2) # ")";
        };

        public func shortText(value : Values.RandomValue) : Text {
            let obj = cast(value);
            name # "(number = " # debug_show (obj.randomNumber) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomObject {
            switch value {
                case (#obj randomObject) randomObject;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomArrayType {
        public let name = "array";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let maxLength = 100_000;
            let length = random.next() % maxLength;
            let randomArray = Prim.Array_init(length, #none) : Values.RandomArray;
            #arr randomArray;
        };

        public func readReferences(source : Values.RandomValue) : [Values.RandomValue] {
            let randomArray = cast(source);
            Values.filterReferences(Array.freeze(randomArray));
        };

        public func append(target : Values.RandomValue, value : Values.RandomValue) : Bool {
            let randomArray = cast(target);
            var index = 0;
            while (index < randomArray.size()) {
                if (Values.isNone(randomArray[index])) {
                    randomArray[index] := value;
                    return true;
                };
                index += 1;
            };
            false;
        };

        public func randomWrite(random : Random.Random, target : Values.RandomValue, value : Values.RandomValue) {
            let randomArray = cast(target);
            if (randomArray.size() == 0) {
                return;
            };
            let index = random.next() % randomArray.size();
            randomArray[index] := value;
        };

        public func randomCheck(random : Random.Random, value : Values.RandomValue) {
            let array = cast(value);
            if (array.size() == 0) {
                return;
            };
            let index = random.next() % array.size();
            let element = array[index];
            let elementType = getType(element);
            if (elementType.name != RandomArrayType.name) {
                elementType.randomCheck(random, element);
            };
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftArray = cast(left);
            let rightArray = cast(right);
            if (leftArray.size() == rightArray.size()) {
                if (leftArray.size() == 0) {
                    return true;
                };
                let testObject = {
                    var randomNumber = 0;
                    var ref1 = #none;
                    var ref2 = #none;
                    checkNumber = 0;
                } : Values.RandomObject;
                let backup = leftArray[0];
                leftArray[0] := #obj testObject;
                let same = isIdentical(leftArray[0], rightArray[0]);
                leftArray[0] := backup;
                same;
            } else {
                false;
            };
        };

        public func hash(value : Values.RandomValue) : Nat {
            let array = cast(value);
            array.size();
        };

        public func text(value : Values.RandomValue) : Text {
            let array = cast(value);
            var result = name # "[";
            var index = 0;
            while (index < array.size()) {
                result #= toShortText(array[index]);
                if (index + 1 < array.size()) {
                    result #= ", ";
                };
                index += 1;
            };
            result # "]";
        };

        public func shortText(value : Values.RandomValue) : Text {
            let array = cast(value);
            "array(size = " # debug_show (array.size()) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomArray {
            switch value {
                case (#arr randomArray) randomArray;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomBlobType {
        public let name = "blob";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let maxLength = 128 * 1024 * 1024;
            let pageSize = 64 * 1024;
            let region = Prim.regionNew();
            ignore Prim.regionGrow(region, Prim.natToNat64(maxLength / pageSize) + 1);
            let blockSize = random.next() % maxLength;
            let randomBlob = Prim.regionLoadBlob(region, 0, blockSize);
            #blob randomBlob;
        };

        public func readReferences(_source : Values.RandomValue) : [Values.RandomValue] {
            [];
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {};

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            ignore cast(value);
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftBlob = cast(left);
            let rightBlob = cast(right);
            leftBlob == rightBlob;
        };

        public func hash(value : Values.RandomValue) : Nat {
            let blob = cast(value);
            blob.size();
        };

        public func text(value : Values.RandomValue) : Text {
            shortText(value);
        };

        public func shortText(value : Values.RandomValue) : Text {
            let blob = cast(value);
            "blob(size = " # debug_show (blob.size()) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomBlob {
            switch value {
                case (#blob randomBlob) randomBlob;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomBoxType {
        public let name = "box";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let randomBigInt : Int = (random.next() * 2 ** 64 + random.next() * 2 ** 32 + random.next()) * -1 * random.next() % 2;
            let randomFloat = Prim.intToFloat(random.next() * -1 * random.next() % 2) / Prim.intToFloat(random.next());
            let randomInt64 = Prim.intToInt64Wrap((random.next() * 2 ** 32 + random.next() * -1 * random.next() % 2));
            let randomBox = {
                var randomBigInt;
                checkBigInt = randomBigInt;
                var randomFloat;
                checkFloat = randomFloat;
                var randomInt64;
                checkInt64 = randomInt64;
            } : Values.RandomBox;
            #box randomBox;
        };

        public func readReferences(_source : Values.RandomValue) : [Values.RandomValue] {
            [];
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {};

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            let box = cast(value);
            assert (box.randomBigInt == box.checkBigInt);
            assert (box.randomFloat == box.checkFloat);
            assert (box.randomInt64 == box.checkInt64);
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftBox = cast(left);
            let rightBox = cast(right);
            leftBox.randomBigInt == rightBox.randomBigInt and leftBox.randomFloat == rightBox.randomFloat and leftBox.randomInt64 == rightBox.randomInt64;
        };

        public func hash(value : Values.RandomValue) : Nat {
            let box = cast(value);
            Prim.nat32ToNat(Prim.intToNat32Wrap(box.randomBigInt));
        };

        public func text(value : Values.RandomValue) : Text {
            shortText(value);
        };

        public func shortText(value : Values.RandomValue) : Text {
            let box = cast(value);
            name # "(box " # debug_show (box.randomBigInt) # ", " # debug_show (box.randomFloat) # ", " # debug_show (box.randomInt64) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomBox {
            switch value {
                case (#box randomBox) randomBox;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomTextType {
        public let name = "text";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            var text = "";
            let maxChunks = 1024;
            let size = random.next() % maxChunks;
            var count = 0;
            while (count < size) {
                text #= "CONCATENATE-" # Prim.charToText(Prim.nat32ToChar(Prim.natToNat32(random.next() % 0xD800)));
                count += 1;
            };
            let randomText = {
                var randomText = text;
                checkText = text;
            } : Values.RandomText;
            #text randomText;
        };

        public func readReferences(_source : Values.RandomValue) : [Values.RandomValue] {
            [];
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {};

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            let text = cast(value);
            assert (text.randomText == text.checkText);
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftText = cast(left);
            let rightText = cast(right);
            leftText.randomText == rightText.randomText;
        };

        func hashText(t : Text) : Nat32 {
            var x : Nat32 = 5381;
            for (char in t.chars()) {
                let c = Prim.charToNat32(char);
                x := ((x << 5) +% x) +% c;
            };
            return x;
        };

        public func hash(value : Values.RandomValue) : Nat {
            let text = cast(value);
            Prim.nat32ToNat(hashText(text.randomText));
        };

        public func text(value : Values.RandomValue) : Text {
            shortText(value);
        };

        public func shortText(value : Values.RandomValue) : Text {
            let text = cast(value);
            name # "(text " # debug_show (text.randomText.size()) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomText {
            switch value {
                case (#text randomText) randomText;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomOptionType {
        public let name = "option";
        public let allocationLimit : ?Nat = null;

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let types = allocationTypes();
            let factory = types[random.next() % types.size()];
            let value = factory.instantiate(random);
            #option(?value);
        };

        public func readReferences(source : Values.RandomValue) : [Values.RandomValue] {
            let option = cast(source);
            switch option {
                case null [];
                case (?value) {
                    if (not Values.isNone(value)) { [value] } else { [] };
                };
            };
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {};

        public func randomCheck(random : Random.Random, value : Values.RandomValue) {
            let option = cast(value);
            switch option {
                case (?value) {
                    getType(value).randomCheck(random, value);
                };
                case _ {};
            };
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftOption = cast(left);
            let rightOption = cast(right);
            switch (leftOption, rightOption) {
                case (null, null) true;
                case (?leftValue, ?rightValue) isIdentical(leftValue, rightValue);
                case _ false;
            };
        };

        public func hash(value : Values.RandomValue) : Nat {
            let option = cast(value);
            switch option {
                case null 0;
                case (?value) toHash(value);
            };
        };

        public func text(value : Values.RandomValue) : Text {
            shortText(value);
        };

        public func shortText(value : Values.RandomValue) : Text {
            let option = cast(value);
            var name = "(option ";
            name #= switch option {
                case null "null";
                case (?value) toShortText(value);
            };
            name # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomOption {
            switch value {
                case (#option randomOption) randomOption;
                case _ Prim.trap("invalid cast");
            };
        };
    };

    public object RandomRegionType {
        public let name = "region";
        public let allocationLimit = ?32752; // 32 * 1024 minus 16 reserved ids

        public func instantiate(random : Random.Random) : Values.RandomValue {
            let PageSize = Prim.natToNat64(64 * 1024);
            let maxPages = 128;
            let pageCount = Prim.natToNat64(1 + random.next() % maxPages);
            let region = Region.new();
            let checkValue = Prim.natToNat8(random.next() % 2 ** 8);
            let result = Region.grow(region, pageCount);
            assert (result != 0xFFFF_FFFF_FFFF_FFFF);
            Region.storeNat8(region, 0, checkValue);
            Region.storeNat8(region, pageCount * PageSize - 2, checkValue);
            #region {
                region;
                checkSize = pageCount;
                checkValue;
            };
        };

        public func readReferences(_source : Values.RandomValue) : [Values.RandomValue] {
            [];
        };

        public func append(_target : Values.RandomValue, _value : Values.RandomValue) : Bool {
            false;
        };

        public func randomWrite(_random : Random.Random, _target : Values.RandomValue, _value : Values.RandomValue) {};

        public func randomCheck(_random : Random.Random, value : Values.RandomValue) {
            let PageSize = Prim.natToNat64(64 * 1024);
            let regionValue = cast(value);
            let region = regionValue.region;
            assert (Region.size(region) == regionValue.checkSize);
            assert (Region.loadNat8(region, 0) == regionValue.checkValue);
            assert (Region.loadNat8(region, regionValue.checkSize * PageSize - 2) == regionValue.checkValue);
        };

        public func identical(left : Values.RandomValue, right : Values.RandomValue) : Bool {
            let leftRegion = cast(left).region;
            let rightRegion = cast(right).region;
            Region.id(leftRegion) == Region.id(rightRegion);
        };

        public func hash(value : Values.RandomValue) : Nat {
            let regionValue = cast(value);
            let region = regionValue.region;
            Prim.nat64ToNat(Region.size(region)) * 2 ** 8 + Prim.nat8ToNat(regionValue.checkValue);
        };

        public func text(value : Values.RandomValue) : Text {
            shortText(value);
        };

        public func shortText(value : Values.RandomValue) : Text {
            let regionValue = cast(value);
            let region = regionValue.region;
            "region(size = " # debug_show (Region.size(region)) # ")";
        };

        func cast(value : Values.RandomValue) : Values.RandomRegion {
            switch value {
                case (#region randomRegion) randomRegion;
                case _ Prim.trap("invalid cast");
            };
        };
    };
};
