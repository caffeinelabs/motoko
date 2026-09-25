/**
 * Module     : symbol.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : stable
 */

import Array "array";
import Common "common";
import Format "format";
import Iter "iter";
import List "list";
import Nat "nat";
import Util "util";
import Version "version";

module {

  type Coordinate = (Nat, Nat);
  type ErrorCorrection = Common.ErrorCorrection;
  type List<T> = List.List<T>;
  type Version = Version.Version;

  public func symbolize(
    version : Version,
    data : List<Bool>
  ) : [var [var Bool]] {
    applyAlignments(
      version,
      applyHardcode(
        version,
        applyTimings(
          version,
          applyFinders(
            version,
            applyData(version, data, init(version))
          )
        )
      )
    )
  };

  func init(version : Version) : [var [var Bool]] {
    let w = Common.width(version);
    let matrix = Array.init<[var Bool]>(w, [var]);
    for i in Iter.range(0, w - 1) {
      matrix[i] := Array.init<Bool>(w, false)
    };
    matrix
  };

  func apply(
    setters : List<(Coordinate, Bool)>,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    List.iter<(Coordinate, Bool)>(setters, func (setter) {
      let i = setter.0.0;
      let j = setter.0.1;
      matrix[i][j] := setter.1
    });
    matrix
  };

  public func freeze(matrix : [var [var Bool]]) : [[Bool]] {
    Array.map<[var Bool], [Bool]>(
      func (row) { Array.freeze<Bool>(row) },
      Array.freeze<[var Bool]>(matrix)
    )
  };

  func applyFinders(
    version : Version,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(finders(version), matrix)
  };

  func finders(version : Version) : List<(Coordinate, Bool)> {
    List.concat<(Coordinate, Bool)>(List.fromArray<List<(Coordinate, Bool)>>([
      finderTL(version),
      finderTR(version),
      finderBL(version)
    ]))
  };

  func finderCoords(version : Version) : List<Coordinate> {
    List.concat<Coordinate>(List.fromArray<List<Coordinate>>([
      finderTLCoords(version),
      finderTRCoords(version),
      finderBLCoords(version)
    ]))
  };

  func finderTL(version : Version) : List<(Coordinate, Bool)> {
    let coords = finderTLCoords(version);
    let pattern = Util.padLeftTo(64, Nat.natToBits(18339425943761911296));
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func finderTLCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let v : Nat = w - 8;
    var coords = List.nil<Coordinate>();
    for i in Iter.range(v, w - 1) {
      for j in Iter.range(v, w - 1) {
        coords := List.push<Coordinate>((i, j), coords)
      }
    };
    coords
  };

  func finderTR(version : Version) : List<(Coordinate, Bool)> {
    let coords = finderTRCoords(version);
    let pattern = Util.padLeftTo(64, Nat.natToBits(9169712971880955648));
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func finderTRCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let r : Nat = w - 8;
    var coords = List.nil<Coordinate>();
    for i in Iter.range(r, w - 1) {
      for j in Iter.range(0, 7) {
        coords := List.push<Coordinate>((i, j), coords)
      }
    };
    coords
  };

  func finderBL(version : Version) : List<(Coordinate, Bool)> {
    let coords = finderBLCoords(version);
    let pattern = Util.padLeftTo(64, Nat.natToBits(71638382592819966));
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func finderBLCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let c : Nat = w - 8;
    var coords = List.nil<Coordinate>();
    for i in Iter.range(0, 7) {
      for j in Iter.range(c, w - 1) {
        coords := List.push<Coordinate>((i, j), coords)
      }
    };
    coords
  };

  func applyTimings(
    version : Version,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(timings(version), matrix)
  };

  func timings(version : Version) : List<(Coordinate, Bool)> {
    List.append<(Coordinate, Bool)>(timingH(version), timingV(version))
  };

  func timingCoords(version : Version) : List<Coordinate> {
    List.append<Coordinate>(timingHCoords(version), timingVCoords(version))
  };

  func timingH(version : Version) : List<(Coordinate, Bool)> {
    let w = Common.width(version);
    let coords = timingHCoords(version);
    let pattern = List.tabulate<Bool>(w - 16, func (n) { n % 2 == 0 });
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func timingHCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let r : Nat = w - 7;
    var coords = List.nil<Coordinate>();
    for j in Iter.range(8, w - 9) {
      coords := List.push<Coordinate>((r, j), coords)
    };
    coords
  };

  func timingV(version : Version) : List<(Coordinate, Bool)> {
    let w = Common.width(version);
    let coords = timingVCoords(version);
    let pattern = List.tabulate<Bool>(w - 16, func (n) { n % 2 == 0 });
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func timingVCoords(version : Version) : List<Coordinate> {
    let coords = timingHCoords(version);
    List.map<Coordinate, Coordinate>(coords, func (a, b) { (b, a) })
  };

  func applyHardcode(
    version : Version,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(hardcode(version), matrix)
  };

  func hardcode(version : Version) : List<(Coordinate, Bool)> {
    let coords = hardcodeCoords(version);
    let pattern = List.singleton<Bool>(true);
    List.zip<Coordinate, Bool>(coords, pattern)
  };

  func hardcodeCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let c : Nat = w - 9;
    List.singleton<Coordinate>((7, c))
  };

  public func applyFormats(
    version : Version,
    level : ErrorCorrection,
    mask : List<Bool>,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(formats(version, level, mask), matrix)
  };

  func formats(
    version : Version,
    level : ErrorCorrection,
    mask : List<Bool>
  ) : List<(Coordinate, Bool)> {
    let coords = formatCoords(version);
    let pattern = Format.encode(level, mask);
    let cycles = List.append<Bool>(pattern, pattern);
    List.zip<Coordinate, Bool>(coords, cycles)
  };

  func formatCoords(version : Version) : List<Coordinate> {
    List.append<Coordinate>(formatHCoords(version), formatVCoords(version))
  };

  func formatHCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let r : Nat = w - 9;
    let c : Nat = w - 8;
    var coords = List.nil<Coordinate>();
    for j in Iter.range(0, 7) {
      coords := List.push<Coordinate>((r, j), coords)
    };
    for j in Iter.range(c, c) {
      coords := List.push<Coordinate>((r, j), coords)
    };
    for j in Iter.range(c + 2, w - 1) {
      coords := List.push<Coordinate>((r, j), coords)
    };
    coords
  };

  func formatVCoords(version : Version) : List<Coordinate> {
    let w = Common.width(version);
    let c : Nat = w - 9;
    var coords = List.nil<Coordinate>();
    for i in Iter.range(0, 6) {
      coords := List.push<Coordinate>((i, c), coords)
    };
    for i in Iter.range(w - 9, w - 8) {
      coords := List.push<Coordinate>((i, c), coords)
    };
    for i in Iter.range(w - 6, w - 1) {
      coords := List.push<Coordinate>((i, c), coords)
    };
    List.rev<Coordinate>(coords)
  };

  public func applyVersions(
    version : Version,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(versions(version), matrix)
  };

  func versions(version : Version) : List<(Coordinate, Bool)> {
    let coords = versionCoords(version);
    let pattern = Version.encode(version);
    let cycles = List.append<Bool>(pattern, pattern);
    List.zip<Coordinate, Bool>(coords, cycles)
  };

  func versionCoords(version : Version) : List<Coordinate> {
    List.append<Coordinate>(versionTRCoords(version), versionBLCoords(version))
  };

  func versionTRCoords(version : Version) : List<Coordinate> {
    if Version.unbox(version) < 7 {
      List.nil<Coordinate>()
    } else {
      func go(n : Nat, a : Nat, b : Nat) : List<Nat> {
        let idxs = Iter.toList<Nat>(Iter.range(a, b));
        List.concat<Nat>(List.replicate<List<Nat>>(n, idxs))
      };
      let w = Common.width(version);
      List.zip<Nat, Nat>(go(3, w - 6, w - 1), go(5, 8, 10))
    }
  };

  func versionBLCoords(version : Version) : List<Coordinate> {
    let coords = versionTRCoords(version);
    List.map<Coordinate, Coordinate>(coords, func (a, b) { (b, a) })
  };

  func applyAlignments(
    version : Version,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(alignment(version), matrix)
  };

  func alignment(version : Version) : List<(Coordinate, Bool)> {
    let n = Common.alignments(version).size() ** 2;
    let m : Nat = if n < 4 { 0 } else { n - 3 };
    let coords = alignmentCoords(version);
    let pattern = Nat.natToBits(33084991);
    let cycles = List.concat<Bool>(List.replicate<List<Bool>>(m, pattern));
    List.zip<Coordinate, Bool>(coords, cycles)
  };

  func alignmentCoords(version : Version) : List<Coordinate> {

    let alignments = Common.alignments(version);
    if alignments.size() == 0 {
      List.nil<Coordinate>()
    } else {

      let a = alignments[0];
      let b = alignments[alignments.size() - 1];
      let reserved = List.fromArray<Coordinate>([(a, b), (b, a), (b, b)]);
      func isReserved(r : Nat, c : Nat) : Bool {
        List.exists<Coordinate>(reserved, func (x, y) {
          (x == r) and (y == c)
        })
      };

      var coords = List.nil<Coordinate>();
      for r in Iter.fromArray<Nat>(alignments) {
        for c in Iter.fromArray<Nat>(alignments) {
          if not isReserved(r, c) {
            for i in Iter.range(r - 2, r + 2) {
              for j in Iter.range(c - 2, c + 2) {
                coords := List.push<Coordinate>((i, j), coords)
              }
            }
          }
        }
      };
      coords
    }
  };

  func applyData(
    version : Version,
    data : List<Bool>,
    matrix : [var [var Bool]]
  ) : [var [var Bool]] {
    apply(path(version, data), matrix)
  };

  func path(version : Version, data : List<Bool>) : List<(Coordinate, Bool)> {
    let coords = pathCoords(version);
    List.zip<Coordinate, Bool>(coords, data)
  };

  public func pathCoords(version : Version) : List<Coordinate> {
    List.foldLeft<Coordinate, List<Coordinate>>(
      patternCoords(version),
      traceCoords(version),
      func ((r, c), coords) {
        List.filter<Coordinate>(coords, func (x, y) {
          not ((x == r) and (y == c))
        })
      }
    )
  };

  func patternCoords(version : Version) : List<Coordinate> {
    List.concat<Coordinate>(List.fromArray<List<Coordinate>>([
      finderCoords(version),
      timingCoords(version),
      hardcodeCoords(version),
      formatCoords(version),
      versionCoords(version),
      alignmentCoords(version)
    ]))
  };

  func traceCoords(version : Version) : List<Coordinate> {

    let w = Common.width(version);
    let t : Nat = w - 7;

    let up = List.concat<Nat>(List.map<Nat, List<Nat>>(
      Iter.toList<Nat>(Iter.range(0, w - 1)),
      func (i) { List.replicate<Nat>(2, i) }
    ));
    let down = List.rev<Nat>(up);

    func rowwise(n : Nat, idxs : List<Nat>) : List<Nat> {
      List.concat<Nat>(List.replicate<List<Nat>>(n * w, idxs))
    };

    func columnwise(idxs : List<Nat>) : List<Nat> {
      List.concat<Nat>(
        List.concat<List<Nat>>(
          List.map<List<Nat>, List<List<Nat>>>(
            List.chunksOf<Nat>(2, idxs),
            func (chunk) {
              List.replicate<List<Nat>>(w, chunk)
            }
          )
        )
      )
    };

    let rows1 = rowwise(w - t, List.append<Nat>(up, down));
    let rows2 = rowwise(6, List.append<Nat>(down, up));

    let cols1 = columnwise(Iter.toList<Nat>(Iter.range(0, t - 1)));
    let cols2 = columnwise(Iter.toList<Nat>(Iter.range(t + 1, w - 1)));

    let coords = List.append<Coordinate>(
      List.zip<Nat, Nat>(rows1, cols1),
      List.zip<Nat, Nat>(rows2, cols2)
    );

    List.filter<Coordinate>(coords, func (x, _) { x != t })
  };

}
