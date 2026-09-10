{-# LANGUAGE ConstraintKinds #-}
{-# LANGUAGE DataKinds #-}
{-# LANGUAGE DeriveFoldable #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE GADTs #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE StandaloneDeriving #-}
{-# LANGUAGE GeneralizedNewtypeDeriving #-}
{-# LANGUAGE TypeApplications #-}
{-# LANGUAGE TypeFamilies #-}
{-# LANGUAGE TupleSections #-}
{-# LANGUAGE UndecidableInstances #-}
{-# LANGUAGE ViewPatterns #-}

{-# OPTIONS_GHC -Wall #-}
{-# OPTIONS_GHC -Wno-missing-methods #-}
{-# OPTIONS_GHC -Wno-name-shadowing #-}
{-# OPTIONS_GHC -Wno-type-defaults #-}
{-# OPTIONS_GHC -Wno-incomplete-patterns #-}
{-# OPTIONS_GHC -Wno-orphans #-}

module Main where

import Control.Applicative
import Test.QuickCheck.Monadic
import Test.Tasty
import Test.Tasty.QuickCheck as QC hiding ((.&.))
import Test.QuickCheck.Unicode
import System.Environment
import qualified Data.Text (null, pack, unpack)
import Data.Maybe
import Data.Bool (bool)
import Data.Proxy
import GHC.Natural
import GHC.TypeLits hiding (Text)
import qualified Data.Word
import qualified Data.Int
import Data.Bits (Bits(..), FiniteBits(..))
import Numeric
import Data.List (intercalate)

import Turtle
import Embedder
-- import Debug.Trace (traceShowId, traceShow)


main :: IO ()
main = do
  putStrLn "Probing embedders..."
  good <- isHealthy embedder
  goodDrun <- isHealthy Drun
  putStrLn . ("Found " <>) . (<> ".") . intercalate " and " . map embedderCommand
               $ [ embedder | good ] <> [ Drun | goodDrun ]
  let tests :: TestTree
      tests = testGroup "Motoko tests" . concat
               $ [ [arithProps, conversionProps, utf8Props, matchingProps] | good ]
               <> [ [encodingProps] | goodDrun ]

  if not (good || goodDrun)
  then putStrLn "No embedder available for testing. Done..."
  else setEnv "TASTY_NUM_THREADS" "1" >> defaultMain tests

arithProps :: TestTree
arithProps = testGroup "Arithmetic/logic"
  [ QC.testProperty "expected failures" $ prop_rejects
  , QC.testProperty "expected successes" $ prop_verifies
  ]

conversionProps :: TestTree
conversionProps = testGroup "Numeric conversions"
  [ QC.testProperty "roundtrip Nat64 Nat Nat64 " $ prop_roundtripNatWNW @64
  , QC.testProperty "roundtrip Nat32 Nat Nat32 " $ prop_roundtripNatWNW @32
  , QC.testProperty "roundtrip Nat16 Nat Nat16 " $ prop_roundtripNatWNW @16
  , QC.testProperty "roundtrip Nat8 Nat Nat8 " $ prop_roundtripNatWNW @8
  , QC.testProperty "roundtrip Int64 Nat Nat64 " $ prop_roundtripIntWNW @64
  , QC.testProperty "roundtrip Int32 Nat Nat32 " $ prop_roundtripIntWNW @32
  , QC.testProperty "roundtrip Int16 Nat Nat16 " $ prop_roundtripIntWNW @16
  , QC.testProperty "roundtrip Int8 Nat Nat8 " $ prop_roundtripIntWNW @8
  , QC.testProperty "modulo Nat Nat64 Nat" $ prop_moduloNWN @64
  , QC.testProperty "modulo Nat Nat32 Nat" $ prop_moduloNWN @32
  , QC.testProperty "modulo Nat Nat16 Nat" $ prop_moduloNWN @16
  , QC.testProperty "modulo Nat Nat8 Nat" $ prop_moduloNWN @8
  ]

utf8Props :: TestTree
utf8Props = testGroup "UTF-8 coding"
  [ QC.testProperty "explode >>> concat roundtrips" $ prop_explodeConcat
  , QC.testProperty "charToText >>> head roundtrips" $ prop_charToText
  , QC.testProperty "length computation" $ prop_textLength
  , QC.testProperty "chunky concat (ropes)" $ prop_ropeConcat
  , QC.testProperty "chunky length (ropes)" $ prop_ropeLength
  , QC.testProperty "chunky iterator (ropes)" $ prop_ropeIterator
  ]

matchingProps :: TestTree
matchingProps = testGroup "Pattern matching" $
  [ QC.testProperty "intra-actor" $ prop_matchStructured ]


-- these require messaging
encodingProps :: TestTree
encodingProps = testGroup "Encoding" $
  [ QC.testProperty "inter-actor" $ withMaxSuccess 20 prop_matchInActor
  , QC.testProperty "encoded-Nat" $ withMaxSuccess 10 prop_matchActorNat
  , QC.testProperty "encoded-Int" $ withMaxSuccess 10 prop_matchActorInt
  , QC.testProperty "encoded-Text" $ withMaxSuccess 10 prop_matchActorText
  ]


withPrim :: Line -> Line
withPrim = (fromString "import Prim \"mo:⛔\";" <>)

runner :: Embedder -> ExitCode -> (Bool -> Bool) -> Turtle.FilePath -> String -> PropertyM IO ()
runner embedder reqOutcome relevant name testCase =
    let as = name <.> "mo"
        wasm = name <.> "wasm"
        fileArg = fromString . encodeString
        script = do Turtle.output as $ withPrim <$> fromString testCase
                    res@(exitCode, _, _) <- procStrictWithErr "moc"
                      (addCompilerArgs embedder ["-no-check-ir", fileArg as]) empty
                    if ExitSuccess == exitCode
                    then (True,) <$> invokeEmbedder embedder wasm
                    else pure (False, res)
    in run script >>= assertOutcomeCheckingFuzz reqOutcome relevant

runScriptNoFuzz    :: Turtle.FilePath -> String -> PropertyM IO ()
runScriptWantFuzz  :: Turtle.FilePath -> String -> PropertyM IO ()
drunScriptNoFuzz   :: Turtle.FilePath -> String -> PropertyM IO ()
drunScriptWantFuzz :: Turtle.FilePath -> String -> PropertyM IO ()

(runScriptNoFuzz, runScriptWantFuzz) = (runEmbedder ExitSuccess id, runEmbedder (ExitFailure 134) not)
    where runEmbedder = runner embedder
(drunScriptNoFuzz, drunScriptWantFuzz) = (runEmbedder ExitSuccess id, runEmbedder (ExitFailure 1) not)
    where runEmbedder = runner Drun

prop_explodeConcat :: UTF8 String -> Property
prop_explodeConcat (UTF8 str) = monadicIO $ do
  let testCase :: String
      testCase = "do { var str = \"\"; for (c in \""
                 <> s <> "\".chars()) { str #= Prim.charToText c }; assert (str == \"" <> s <> "\") }"

      s = concatMap escape str
  runScriptNoFuzz "explodeConcat" testCase

-- TODO: why can't we use Test.QuickCheck.Unicode.Unicode? (see https://github.com/bos/quickcheck-unicode/issues/5)
newtype UTF8 a = UTF8 a deriving Show

instance Arbitrary (UTF8 String) where
  arbitrary = UTF8 <$> string1

instance Arbitrary (UTF8 Char) where
  arbitrary = UTF8 <$> Test.QuickCheck.Unicode.char

hex :: Int -> String
hex = (`showHex` "")

escape :: Char -> String
escape ch | '\\' `elem` show ch = "\\u{" <> hex (fromEnum ch) <> "}"
escape '"' = "\\\""
escape ch = pure ch

prop_charToText :: UTF8 Char -> Property
prop_charToText (UTF8 char) = monadicIO $ do
  let testCase = "assert (switch ((Prim.charToText '"
                 <> c <> "').chars().next()) { case (?'" <> c <> "') true; case _ false })"

      c = escape char
  runScriptNoFuzz "charToText" testCase

prop_textLength :: UTF8 [Char] -> Property
prop_textLength (UTF8 text) = monadicIO $ do
  let testCase = "assert(\"" <> (text >>= escape) <> "\".size() == " <> show (length text) <> ")"
  runScriptNoFuzz "textLength" testCase

data Rope a = EmptyChunk | Chunk a | UTF8Chunk a | LongChunk a | Rope a `Rope` Rope a deriving (Eq, Show, Foldable)

instance Semigroup (Rope a) where
  (<>) = Rope

instance Monoid (Rope a) where
  mempty = EmptyChunk

instance Arbitrary (Rope String) where
  arbitrary = frequency [ (2, pure mempty)
                        , (4, Chunk <$> arbitrary)
                        , (5, elements [ UTF8Chunk "Медве́ди хо́дят по у́лицам. Коне́чно, э́то непра́вда!"
                                       , UTF8Chunk "Boci, boci tarka, se füle, se farka, oda megyünk lakni, ahol tejet kapni."
                                       , UTF8Chunk "十年树木，百年树人" ])
                        , (7, pure $ LongChunk "The quick brown fox jumps over the lazy dog")
                        , (3, Rope <$> arbitrary <*> arbitrary) ]

instance Semigroup (MOTerm String) where
  (<>) = Concat

instance Monoid (MOTerm String) where
  mempty = Text mempty

asString :: Rope String -> String
asString = foldMap id

asMot :: Rope String -> MOTerm String
asMot = foldMap Text

prop_ropeConcat :: Rope String -> Property
prop_ropeConcat rope = monadicIO $ do
  let testCase = "assert (" <> ropeMot <> " == " <> string <> ")"
      string = unparseMO (Text (asString rope))
      ropeMot = unparseMO (asMot rope)
  runScriptNoFuzz "ropeConcat" testCase

prop_ropeLength :: Rope String -> Property
prop_ropeLength rope = monadicIO $ do
  let testCase = "assert (" <> ropeMot <> ".size() == " <> show len <> ")"
      len = length (asString rope)
      ropeMot = unparseMO (asMot rope)
  runScriptNoFuzz "ropeLength" testCase

prop_ropeIterator :: Rope String -> Property
prop_ropeIterator rope = monadicIO $ do
  let testCase = "func same(c : ?Char, d : Char) : Bool = switch c { case (?cc) { cc == d }; case null false };"
              <> "let i = (" <> ropeMot <> ").chars();"
              <> concatMap testStep string
              <> "assert (switch (i.next()) { case null true; case _ false })"
      testStep c = "assert (same(i.next(), '" <> escape c <> "'));"
      string = asString rope
      ropeMot = unparseMO (asMot rope)
  runScriptNoFuzz "ropeLength" testCase


assertOutcomeCheckingFuzz ::
    ExitCode -> (Bool -> Bool) -> (Bool, (ExitCode, Text, Text)) -> PropertyM IO ()
assertOutcomeCheckingFuzz outcome relevant (compiled, (exitCode, out, err)) = do
  let fuzzErr = not $ Data.Text.null err
      fuzzErrRelevant = relevant fuzzErr
  when (fuzzErr && fuzzErrRelevant) $ do
    monitor (counterexample "STDERR:")
    monitor (counterexample . Data.Text.unpack $ err)
  let fuzzOut = not $ Data.Text.null out
  when fuzzOut $ do
    monitor (counterexample "STDOUT:")
    monitor (counterexample . Data.Text.unpack $ out)
  assert (not $ outcome /= exitCode || (if compiled then fuzzErrRelevant else fuzzErr) || fuzzOut)

assertSuccessNoFuzz :: (Bool, (ExitCode, Text, Text)) -> PropertyM IO ()
assertSuccessNoFuzz = assertOutcomeCheckingFuzz ExitSuccess id

newtype Failing = Failing (MOTerm Integer)

instance Arbitrary Failing where
  arbitrary = Failing <$> suchThat (resize 5 arbitrary) (\(evaluate @Integer -> res) -> null res)
  shrink (Failing term) = do
    term' <- shrink term
    Nothing <- pure $ evaluate term'
    pure $ Failing term'

unparseFailing :: Failing -> String
unparseFailing (Failing t) = "let _ = " ++ unparseMO t ++ ";"

instance Show Failing where
    show = unparseFailing

prop_rejects :: Failing -> Property
prop_rejects f = monadicIO $ runScriptWantFuzz "fails" (unparseFailing f)

newtype TestCase = TestCase (MOTerm Integer)

unparseTestCase :: TestCase -> String
unparseTestCase (TestCase t) =
  let actual = unparseMO t in
  let Just expected = evaluate @Integer t in
  "assert (" ++ actual ++ " == " ++ show expected ++ ");"

instance Show TestCase where show = unparseTestCase

instance Arbitrary TestCase where
  arbitrary = TestCase <$> suchThat (resize 5 arbitrary) (isJust . evaluate @Integer)
  shrink (TestCase term) = do
    term' <- shrink term
    Just _ <- pure $ evaluate term'
    pure $ TestCase term'

newtype TestCases = TestCases [TestCase]

instance Show TestCases where
  show (TestCases tcs) = unlines $ map unparseTestCase tcs

instance Arbitrary TestCases where
  arbitrary = TestCases <$> vector 10
  shrink (TestCases tcs) = TestCases <$> shrink tcs

prop_verifies :: TestCases -> Property
prop_verifies (TestCases tcs) = monadicIO $ do
  Turtle.output "tests.mo" $ msum $ pure (withPrim mempty) : [ fromString (unparseTestCase tc) | tc <- tcs ]
  res@(exitCode, _, _) <- procStrictWithErr "moc"
           (addCompilerArgs embedder ["-no-check-ir", "tests.mo"]) empty
  res' <- if exitCode == ExitSuccess
          then (True,) <$> procStrictWithErr (embedderCommand embedder) (addEmbedderArgs embedder ["tests.wasm"]) empty
          else pure (False, res)
  assertSuccessNoFuzz res'


newtype ConversionTestNat n = ConversionTestNat (Neuralgic (BitLimited n Natural)) deriving (Show, Arbitrary)
newtype ConversionTestInt n = ConversionTestInt (Neuralgic (BitLimited n Integer)) deriving (Show, Arbitrary)

prop_roundtripNatWNW :: WordLike n => ConversionTestNat n -> Property
prop_roundtripNatWNW (ConversionTestNat n) = do
    let expected = Neuralgic n
    let actual = ConvertNatToNatN (ConvertNatNToNat expected)
    let testCase = "assert(" <> unparseMO (actual `Equals` expected) <> ")"
    monadicIO $ runScriptNoFuzz "roundtripWNW" testCase
prop_roundtripIntWNW :: WordLike n => ConversionTestInt n -> Property
prop_roundtripIntWNW (ConversionTestInt n) = do
    let expected = Neuralgic n
    let actual = ConvertIntToIntN (ConvertIntNToInt expected)
    let testCase = "assert(" <> unparseMO (actual `Equals` expected) <> ")"
    monadicIO $ runScriptNoFuzz "roundtripWNW" testCase


newtype ModuloTest (n :: Nat) = ModuloTest (MOTerm (BitLimited n Natural)) deriving Show

instance WordLike n => Arbitrary (ModuloTest n) where
  arbitrary = ModuloTest . ConvertNatToNatN @n . Neuralgic <$> (arbitrary :: Gen (Neuralgic Natural))

prop_moduloNWN :: forall n . KnownNat n => ModuloTest n -> Property
prop_moduloNWN (ModuloTest term@(ConvertNatToNatN (Neuralgic m))) = monadicIO $ runScriptNoFuzz "moduloNWN" testCase
  where m' = evalN m .&. maskFor term
        testCase = "assert(" <> unparseMO (ConvertNatNToNat term)
                <> " == " <> show m' <> ")"

data Matching where
  Matching :: (AnnotLit t, MOValue t, Show t, Evaluatable t, Arbitrary (MOTerm t)) => (MOTerm t, t) -> Matching

deriving instance Show Matching


instance Arbitrary Matching where
  arbitrary = oneof [ realise Matching <$> gen @Bool
                    , realise Matching <$> gen @(Bool, Bool)
                    , realise Matching <$> gen @(Bool, Integer)
                    , realise Matching <$> gen @((Bool, Natural), Integer)
                    , realise Matching <$> gen @(BitLimited 8 Natural, BitLimited 8 Integer)
                    , realise Matching <$> gen @(Maybe Integer)
                    ]
    where gen :: (Arbitrary (MOTerm a), Evaluatable a) => Gen (MOTerm a, Maybe a)
          gen = (do term <- arbitrary
                    let val = evaluate term
                    pure (term, val)) `suchThat` (isJust . snd)
          realise f (tm, Just v) = f (tm, v)
  shrink (Matching (term, _val)) = do
    term' <- shrink term
    Just val' <- pure $ evaluate term'
    pure $ Matching (term', val')

prop_matchStructured :: Matching -> Property
prop_matchStructured (Matching a) = locally a

locally :: (AnnotLit t, MOValue t) => (MOTerm t, t) -> Property
locally (tm, v) = monadicIO $ do
  let testCase = "assert (switch (" <> expr <> ") { case (" <> eval'd <> ") true; case _ false })"

      eval'd = unparse v
      expr = unparseMO tm
  runScriptNoFuzz "matchLocally" testCase

prop_matchInActor :: Matching -> Property
prop_matchInActor (Matching a) = mobile a

mobile :: (AnnotLit t, MOValue t) => (MOTerm t, t) -> Property
mobile (tm, v) = monadicIO $ do
  let testCase = "actor { public func match (b : " <> typed <> ") : async () { assert (switch b { case (" <> eval'd <> ") true; case _ false }) }; public func go () : async () { let res = await match (" <> expr <> " : " <> typed <> "); return res } };"

      eval'd = unparse v
      typed = unparseType v
      expr = unparseMO tm
  drunScriptNoFuzz "matchMobile" testCase


prop_matchActorNat :: Neuralgic Natural -> Property
prop_matchActorNat nat = monadicIO $ do
    let testCase = format ("actor { public func match (n : Nat) : async () { assert (switch n { case ("%d%") true; case _ false }) }; public func go () : async () { let res = await match ("%d%" : Nat); return res } };") eval'd eval'd
        eval'd = evalN nat
    drunScriptNoFuzz "matchActorNat" (Data.Text.unpack testCase)

prop_matchActorInt :: Neuralgic Integer -> Property
prop_matchActorInt int = monadicIO $ do
    let testCase = format ("actor { public func match (i : Int) : async () { assert (switch i { case ("%d%") true; case _ false }) }; public func go () : async () { let res = await match ("%d%" : Int); return res } };") eval'd eval'd
        eval'd = evalN int
    drunScriptNoFuzz "matchActorInt" (Data.Text.unpack testCase)

prop_matchActorText :: UTF8 String -> Property
prop_matchActorText (UTF8 text) = monadicIO $ do
    let testCase = format ("actor { public func match (i : Text) : async () { assert (switch i { case (\""%s%"\") true; case _ false }) }; public func go () : async () { let res = await match (\""%s%"\" : Text); return res } };") eval'd eval'd
        eval'd = Data.Text.pack $ text >>= escape
    drunScriptNoFuzz "matchActorText" (Data.Text.unpack testCase)

-- instances of MOValue describe "ground values" in
-- Motoko. These can appear in patterns and have
-- well-defined Motoko type.
--
class MOValue a where
  unparseType :: a -> String
  unparse :: a -> String

instance MOValue Bool where
  unparseType _ = "Bool"
  unparse = unparseMO . Bool

instance MOValue Integer where
  unparseType _ = "Int"
  unparse = show

instance MOValue Natural where
  unparseType _ = "Nat"
  unparse = show

instance KnownNat n => MOValue (BitLimited n Natural) where
  unparseType _ = "Nat" <> bitWidth (Proxy @n)
  unparse (NatN a) = annot (Five @(BitLimited n Natural)) (show a)

instance KnownNat n => MOValue (BitLimited n Integer) where
  unparseType _ = "Int" <> bitWidth (Proxy @n)
  unparse (IntN a) = annot (Five @(BitLimited n Integer)) (show a)

instance (MOValue a, MOValue b) => MOValue (a, b) where
  unparseType (a, b) = "(" <> unparseType a <> ", " <> unparseType b <> ")"
  unparse (a, b) = "(" <> unparse a <> ", " <> unparse b <> ")"

instance (MOValue a, MOValue b, MOValue c) => MOValue (a, b, c) where
  unparseType (a, b, c) = "(" <> unparseType a <> ", " <> unparseType b <> ", " <> unparseType c <> ")"
  unparse (a, b, c) = "(" <> unparse a <> ", " <> unparse b <> ", " <> unparse c <> ")"

instance MOValue a => MOValue (Maybe a) where
  unparseType Nothing = "Null"
  unparseType (Just a) = "?" <> unparseType a
  unparse Nothing = "null"
  unparse (Just a) = "?" <> unparse a

-- wiggle room around an *important value*
-- think of it as a "fuzz"
--
data Off = TwoLess | OneLess | OneMore | TwoMore
 deriving (Enum, Eq, Ord, Show)

instance Arbitrary Off where
  arbitrary = elements [TwoLess .. TwoMore]

-- Below data structure tries to focus test case generation of
-- numeric values to *neuralgic points* of the numeric line, namely
-- areas where (from our white-box knowledge of Motoko's inner
-- workings) representation changes are expected to happen. These
-- are mostly at a power-of-two boundary and around it, e.g. we have
-- signed 30 bit numbers (`Int` and `Nat`) that have a compact representation
-- and out of this range the number gets heap-allocated.
--
data Neuralgic t
  = LargeNeg
  | AroundNeg Integer
  | Around0
  | AroundPos Integer
  | LargePos
  | Offset (Neuralgic t) Off
 deriving (Eq, Ord, Show)

instance Arbitrary (Neuralgic Integer) where
  arbitrary = frequency [ (5, elements [LargeNeg, AroundNeg 63, AroundNeg 30, Around0, AroundPos 30, AroundPos 63, LargePos])
                        , (8, Offset <$> arbitrary <*> arbitrary)]


guardedFrom :: (Arbitrary (Neuralgic a), Integral a) =>
              (Integer -> Maybe a) -> [Neuralgic a] -> Gen (Neuralgic a)
pred `guardedFrom` set = frequency [ (5, elements set)
                                   , (3, Offset <$> arbitrary <*> arbitrary)]
                         `suchThat` (isJust . guardedEvalN pred)
infix 1 `guardedFrom`

instance Arbitrary (Neuralgic Natural) where
  arbitrary =  (\n -> if n >= 0 then pure (fromIntegral n) else Nothing)
               `guardedFrom` [Around0, AroundPos 30, AroundPos 63, LargePos, AroundPos 77]

instance KnownNat n => Arbitrary (Neuralgic (BitLimited n Natural)) where
  arbitrary = fmap NatN <$> trapNat bits `guardedFrom` menu bits
    where bits = natVal (Proxy @n)
          menu 8 = [Around0, AroundPos 3, AroundPos 5, AroundPos 8]
          menu 16 = [Around0, AroundPos 3, AroundPos 5, AroundPos 8, AroundPos 13, AroundPos 16]
          menu 32 = [Around0, AroundPos 8, AroundPos 13, AroundPos 16, AroundPos 23, AroundPos 32]
          menu 64 = [Around0, AroundPos 8, AroundPos 13, AroundPos 23, AroundPos 31, AroundPos 47, AroundPos 64]


instance KnownNat n => Arbitrary (Neuralgic (BitLimited n Integer)) where
  arbitrary = fmap IntN <$> trapInt bits `guardedFrom` menu bits
    where bits = natVal (Proxy @n)
          menu 8 = [Around0, AroundNeg 3, AroundNeg 5, AroundNeg 7, AroundPos 3, AroundPos 5, AroundPos 7]
          menu 16 = [Around0, AroundNeg 3, AroundNeg 7, AroundNeg 10, AroundNeg 15, AroundPos 3, AroundPos 8, AroundPos 10, AroundPos 15]
          menu 32 = [Around0, AroundNeg 3, AroundNeg 17, AroundNeg 27, AroundNeg 31, AroundPos 3, AroundPos 18, AroundPos 25, AroundPos 31]
          menu 64 = [Around0, AroundNeg 9, AroundNeg 27, AroundNeg 51, AroundNeg 63, AroundPos 10, AroundPos 28, AroundPos 55, AroundPos 63]


data MOTerm :: * -> * where
  -- Comparisons
  NotEqual, Equals, GreaterEqual, Greater, LessEqual, Less
    :: (AnnotLit a, Evaluatable a, Arbitrary (MOTerm a)) =>
       MOTerm a -> MOTerm a -> MOTerm Bool
  -- Short-circuit
  ShortAnd, ShortOr
    :: MOTerm Bool -> MOTerm Bool -> MOTerm Bool
  -- Boolean
  Not :: MOTerm Bool -> MOTerm Bool
  Bool :: Bool -> MOTerm Bool
  -- Bitwise
  Complement :: MOTerm (BitLimited n a) -> MOTerm (BitLimited n a)
  Or, And, Xor, RotL, RotR, ShiftL, ShiftR
    :: MOTerm (BitLimited n a) -> MOTerm (BitLimited n a) -> MOTerm (BitLimited n a)
  PopCnt, Clz, Ctz :: MOTerm (BitLimited n a) -> MOTerm (BitLimited n a)
  -- Arithmetic
  Pos, Neg, Abs :: MOTerm a -> MOTerm a
  Add, Sub, Mul, Div, Mod, Pow :: MOTerm a -> MOTerm a -> MOTerm a
  -- Wrapping Arithmetic
  WrapAdd, WrapSub, WrapMul, WrapPow
    :: MOTerm (BitLimited n a) -> MOTerm (BitLimited n a) -> MOTerm (BitLimited n a)
  -- Numeric
  Lit :: Integer -> MOTerm a
  Neuralgic :: Neuralgic a -> MOTerm a
  Five :: MOTerm a
  -- Text
  Text :: String -> MOTerm String
  Concat :: MOTerm String -> MOTerm String -> MOTerm String
  -- Conditional
  IfThenElse :: MOTerm a -> MOTerm a -> MOTerm Bool -> MOTerm a
  -- Conversion
  ConvertNatural :: MOTerm Natural -> MOTerm Integer
  ConvertNatToNatN :: WordLike n => MOTerm Natural -> MOTerm (BitLimited n Natural)
  ConvertNatNToNat :: WordLike n => MOTerm (BitLimited n Natural) -> MOTerm Natural
  ConvertIntToIntN :: WordLike n => MOTerm Integer -> MOTerm (BitLimited n Integer)
  ConvertIntNToInt :: WordLike n => MOTerm (BitLimited n Integer) -> MOTerm Integer
  -- Constructors (intro forms)
  Pair ::
    ( AnnotLit a, AnnotLit b
    , Evaluatable a, Evaluatable b
    , Arbitrary (MOTerm a), Arbitrary (MOTerm b)
    ) => MOTerm a -> MOTerm b -> MOTerm (a, b)
  Triple ::
    ( AnnotLit a, AnnotLit b, AnnotLit c
    , Evaluatable a, Evaluatable b, Evaluatable c
    , Arbitrary (MOTerm a), Arbitrary (MOTerm b), Arbitrary (MOTerm c)
    ) => MOTerm a -> MOTerm b -> MOTerm c -> MOTerm (a, b, c)
  Array :: MOTerm a -> MOTerm [a] -- not matchable!
  Null :: MOTerm (Maybe a)
  Some :: (AnnotLit a, Evaluatable a, Arbitrary (MOTerm a)) => MOTerm a -> MOTerm (Maybe a)
  -- Variants, Objects (TODO)

shrinkRel2 :: (Arbitrary a, Arbitrary b) => (a -> b -> c) -> a -> b -> [c]
shrinkRel2 f a b = concat
    [ (\a -> f a b) <$> shrink a
    , (\b -> f a b) <$> shrink b
    ]

shrinkRel3 :: (Arbitrary a, Arbitrary b, Arbitrary c) => (a -> b -> c -> d) -> a -> b -> c -> [d]
shrinkRel3 f a b c = concat
    [ (\a -> f a b c) <$> shrink a
    , (\b -> f a b c) <$> shrink b
    , (\c -> f a b c) <$> shrink c
    ]

shrinkOp2 :: Arbitrary a => (a -> a -> a) -> a -> a -> [a]
shrinkOp2 f a b = a : b : shrinkRel2 f a b

shrinkOp1 :: Arbitrary a => (a -> a) -> a -> [a]
shrinkOp1 f a = a : (f <$> shrink a)

subShrink :: Arbitrary (MOTerm t) => MOTerm t -> [MOTerm t]
subShrink Five = []
subShrink (Lit _) = []
subShrink (Neuralgic _) = []
subShrink (Pos n) = shrinkOp1 Pos n
subShrink (Neg n) = shrinkOp1 Neg n
subShrink (Abs n) = shrinkOp1 Abs n
subShrink (a `Add` b) = shrinkOp2 Add a b
subShrink (a `Sub` b) = shrinkOp2 Sub a b
subShrink (a `Mul` b) = shrinkOp2 Mul a b
subShrink (a `Div` b) = shrinkOp2 Div a b
subShrink (a `Mod` b) = shrinkOp2 Mod a b
subShrink (a `Or` b) = shrinkOp2 Or a b
subShrink (a `And` b) = shrinkOp2 And a b
subShrink (a `Xor` b) = shrinkOp2 Xor a b
subShrink (a `RotL` b) = shrinkOp2 RotL a b
subShrink (a `RotR` b) = shrinkOp2 RotR a b
subShrink (a `ShiftL` b) = shrinkOp2 ShiftL a b
subShrink (a `ShiftR` b) = shrinkOp2 ShiftR a b
subShrink (a `WrapAdd` b) = shrinkOp2 WrapAdd a b
subShrink (a `WrapSub` b) = shrinkOp2 WrapSub a b
subShrink (a `WrapMul` b) = shrinkOp2 WrapMul a b
subShrink (a `WrapPow` b) = shrinkOp2 WrapPow a b
subShrink (PopCnt n) = shrinkOp1 PopCnt n
subShrink (Clz n) = shrinkOp1 Clz n
subShrink (Ctz n) = shrinkOp1 Ctz n
subShrink (Complement n) = shrinkOp1 Complement n
subShrink (ConvertNatural a) = ConvertNatural <$> shrink a
subShrink (ConvertNatNToNat a) = ConvertNatNToNat <$> shrink a
subShrink (ConvertNatToNatN a) = ConvertNatToNatN <$> shrink a
subShrink (ConvertIntNToInt a) = ConvertIntNToInt <$> shrink a
subShrink (ConvertIntToIntN a) = ConvertIntToIntN <$> shrink a
subShrink (IfThenElse a _ (Bool True)) = [a]
subShrink (IfThenElse _ b (Bool False)) = [b]
subShrink (IfThenElse a b c) = [a, b] <> shrinkRel3 IfThenElse a b c
subShrink (a `NotEqual` b) = shrinkRel2 NotEqual a b
subShrink (a `Equals` b) = shrinkRel2 Equals a b
subShrink (a `GreaterEqual` b) = shrinkRel2 GreaterEqual a b
subShrink (a `Greater` b) = shrinkRel2 Greater a b
subShrink (a `LessEqual` b) = shrinkRel2 LessEqual a b
subShrink (a `Less` b) = shrinkRel2 Less a b
subShrink (a `ShortAnd` b) = shrinkRel2 ShortAnd a b
subShrink (a `ShortOr` b) = shrinkRel2 ShortOr a b
subShrink (Not a) = Not <$> shrink a
subShrink (Bool False) = []
subShrink (Bool True) = []
subShrink (a `Pair` b) = shrinkRel2 Pair a b
subShrink (Triple a b c) = shrinkRel3 Triple a b c
subShrink Null = []
subShrink (Some a) = Null : (Some <$> shrink a)
subShrink (Text a) = Text <$> shrink a
subShrink (a `Concat` b) = shrinkOp2 Concat a b

deriving instance Show (MOTerm t)


arithTerm :: Arbitrary (MOTerm t) =>
    Int -> [(Int, Gen (MOTerm t))]
arithTerm n =
    [ (n, resize (n `div` 3) $ Add <$> arbitrary <*> arbitrary)
    , (n, resize (n `div` 3) $ Sub <$> arbitrary <*> arbitrary)
    , (n, resize (n `div` 3) $ Mul <$> arbitrary <*> arbitrary)
    , (n, resize (n `div` 3) $ Div <$> arbitrary <*> arbitrary)
    , (n, resize (n `div` 3) $ Mod <$> arbitrary <*> arbitrary)
    , (n, resize (n `div` 4) $ IfThenElse <$> arbitrary <*> arbitrary <*> arbitrary)
    , (n `div` 5 , resize (n `div` 3) $ Pow <$> arbitrary <*> ((`Mod` Five) <$> arbitrary))
    ]

bitwiseTerm :: WordLike n => Arbitrary (MOTerm (BitLimited n i)) => Int -> [(Int, Gen (MOTerm (BitLimited n i)))]
bitwiseTerm n =
    [ (n `div` 5, resize (n `div` 3) $ Or <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ And <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ Xor <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ RotL <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ RotR <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ ShiftL <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ ShiftR <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ WrapAdd <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ WrapSub <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ WrapMul <$> arbitrary <*> arbitrary)
    , (n `div` 5, resize (n `div` 3) $ WrapPow <$> arbitrary <*> ((`Mod` Five) <$> arbitrary))
    , (n `div` 5, PopCnt <$> arbitrary)
    , (n `div` 5, Clz <$> arbitrary)
    , (n `div` 5, Ctz <$> arbitrary)
    , (n `div` 5, Complement <$> arbitrary)
    ]

-- generate reasonably formed trees from smaller subterms
--
reasonablyShaped :: (Arbitrary (Neuralgic a), AnnotLit a, Evaluatable a)
                 => (Int -> [(Int, Gen (MOTerm a))])
                 -> Gen (MOTerm a)
reasonablyShaped sub = sized $ \(succ -> n) -> frequency $
                       (30 `div` n, Neuralgic <$> arbitrary)
                       : if n > 1 then sub n else []

instance WordLike n => Arbitrary (MOTerm (BitLimited n Natural)) where
  arbitrary = reasonablyShaped $ \n ->
    arithTerm n <>
    bitwiseTerm n
  shrink (Lit _) = []
  shrink x = [ Lit (fromIntegral x) | Just x <- pure $ evaluate x ] <> subShrink x

instance WordLike n => Arbitrary (MOTerm (BitLimited n Integer)) where
  arbitrary = reasonablyShaped $ \n ->
    arithTerm n <>
    bitwiseTerm n
  shrink (Lit _) = []
  shrink x = [ Lit (fromIntegral x) | Just x <- pure $ evaluate x ] <> subShrink x

instance (AnnotLit a, AnnotLit b, Evaluatable a, Evaluatable b, Arbitrary (MOTerm a), Arbitrary (MOTerm b)) => Arbitrary (MOTerm (a, b)) where
  arbitrary = scale (`quot` 2) $ Pair <$> arbitrary <*> arbitrary
  shrink (Pair x y) = shrinkRel2 Pair x y

instance (AnnotLit a, AnnotLit b, AnnotLit c, Evaluatable a, Evaluatable b, Evaluatable c, Arbitrary (MOTerm a), Arbitrary (MOTerm b), Arbitrary (MOTerm c))
    => Arbitrary (MOTerm (a, b, c)) where
  arbitrary = scale (`quot` 3) $ Triple <$> arbitrary <*> arbitrary <*> arbitrary
  shrink (Triple x y z) = shrinkRel3 Triple x y z

instance (AnnotLit a, Evaluatable a, Arbitrary (MOTerm a)) => Arbitrary (MOTerm (Maybe a)) where
  arbitrary = frequency [(1, pure Null), (10, Some <$> arbitrary)]
  shrink Null = []
  shrink (Some t) = Null : [ Some t' | t' <- shrink t ]

instance Arbitrary (MOTerm Bool) where
  arbitrary = sized $ \(succ -> n) -> -- TODO: use frequency?
    oneof $ (Bool <$> arbitrary) : if n <= 1 then [] else
    [ resize (n `div` 3) $ elements [NotEqual @Integer, Equals, GreaterEqual, Greater, LessEqual, Less] <*> arbitrary <*> arbitrary
    , resize (n `div` 5) $ elements [ShortAnd, ShortOr] <*> arbitrary <*> arbitrary
    , resize (n `div` 2) $ Not <$> arbitrary
    ]
  shrink (Bool _) = []
  shrink x = [ Bool x | Just x <- pure $ evaluate x ] <> subShrink x

instance Arbitrary (MOTerm Natural) where
  arbitrary = reasonablyShaped $ \n ->
    arithTerm n <>
    [ (n `div` 2, ConvertNatNToNat <$> (arbitrary @(MOTerm Nat8)))
    , (n `div` 2, ConvertNatNToNat <$> (arbitrary @(MOTerm Nat16)))
    , (n `div` 2, ConvertNatNToNat <$> (arbitrary @(MOTerm Nat32)))
    , (n `div` 2, ConvertNatNToNat <$> (arbitrary @(MOTerm Nat64)))
    ]
  shrink (Lit _) = []
  shrink x = [ Lit (fromIntegral x) | Just x <- pure $ evaluate x ] <> subShrink x

instance Arbitrary (MOTerm Integer) where
  arbitrary = reasonablyShaped $ \n ->
    arithTerm n <>
    [ (n, resize (n `div` 2) $ Pos <$> arbitrary)
    , (n, resize (n `div` 2) $ Neg <$> arbitrary)
    , (n, resize (n `div` 2) $ Abs <$> arbitrary)
    , (5*n, ConvertNatural <$> arbitrary)
    , (n `div` 3, ConvertIntNToInt <$> (arbitrary @(MOTerm Int8)))
    , (n `div` 3, ConvertIntNToInt <$> (arbitrary @(MOTerm Int16)))
    , (n `div` 3, ConvertIntNToInt <$> (arbitrary @(MOTerm Int32)))
    , (n `div` 3, ConvertIntNToInt <$> (arbitrary @(MOTerm Int64)))
    ]
  shrink (Lit _) = []
  shrink x = [ Lit (fromIntegral x) | Just x <- pure $ evaluate x ] <> subShrink x

instance Num a => Num (Maybe a) where
  (+) = liftA2 (+)
  (-) = liftA2 (-)
  (*) = liftA2 (*)
  abs = liftA abs
  signum = liftA signum
  fromInteger = pure . fromInteger

instance Real a => Real (Maybe a) where
  toRational = undefined

instance Enum a => Enum (Maybe a)

instance (Enum (Maybe a), Eq a, Integral a) => Integral (Maybe a) where
  _ `quotRem` Just 0 = (Nothing, Nothing)
  a `quotRem` b = case liftA2 quotRem a b of
                    Just (pure -> x, pure -> y) -> (x, y)
                    _ -> (Nothing, Nothing)


evalO :: Num a => Off -> a -> a
evalO TwoLess = \n -> n - 2
evalO OneLess = \n -> n - 1
evalO OneMore = (+1)
evalO TwoMore = (+2)

evalN :: Num a => Neuralgic t -> a
evalN LargeNeg = fromIntegral (minBound :: Int) - 111
evalN (AroundNeg n) = - 2 ^ n
evalN Around0 = 0
evalN (AroundPos n) = 2 ^ n - 1
evalN LargePos = fromIntegral (maxBound :: Int) + 333
evalN (Offset (evalN -> n) (evalO -> o)) = o n


guardedEvalN :: Integral a => (Integer -> Maybe a) -> Neuralgic a -> Maybe a
guardedEvalN g (Offset (guardedEvalN g -> n) (evalO -> o)) = g =<< (o . toInteger) <$> n
guardedEvalN g (g . evalN -> res) = res

noExponentRestriction, nonNegativeExponentRestriction, defaultExponentRestriction :: (Ord a, Num a) => a -> Maybe ()
noExponentRestriction _ = pure ()
nonNegativeExponentRestriction = guard . (>= 0)
defaultExponentRestriction = guard . ((&&) <$> (>= 0) <*> (< 5))

class Restricted a where
  substractable :: Maybe a -> Maybe a -> Bool
  exponentiable :: (Num a, Ord a) => a -> Maybe ()
  exponentiable = defaultExponentRestriction

instance Restricted Integer where substractable _ _ = True
instance Restricted Natural where substractable a b = isJust $ do m <- a; n <- b; guard $ m >= n
instance KnownNat n => Restricted (BitLimited n Natural) where
  substractable a b = isJust $ do NatN m <- a; NatN n <- b; guard $ m >= n
  exponentiable = if natVal (Proxy @n) <= 8
                  then noExponentRestriction
                  else defaultExponentRestriction
instance KnownNat n => Restricted (BitLimited n Integer) where
  exponentiable = if natVal (Proxy @n) <= 8
                  then nonNegativeExponentRestriction
                  else defaultExponentRestriction

class Ord a => Evaluatable a where
  evaluate :: MOTerm a -> Maybe a


data BitLimited (n :: Nat) (a :: *) where
  NatN :: KnownNat n => Natural -> BitLimited n Natural
  IntN :: KnownNat n => Integer -> BitLimited n Integer

deriving instance Show (BitLimited n a)
deriving instance Eq (BitLimited n a)
deriving instance Ord (BitLimited n a)


-- for a BitLimited n Word, the instances of WordView provide the
-- change of perspective to the built-in Word types' semantics
--
class WordView a where
  type WordType a :: *
  toWord :: a -> WordType a
  fromWord :: WordType a -> a
  lift1 :: (WordType a -> WordType a) -> a -> a
  lift1 f (fromWord . f . toWord -> res) = res
  lift1b :: (WordType a -> b) -> a -> b
  lift1b f (f . toWord -> res) = res
  lift2 :: (WordType a -> WordType a -> WordType a) -> a -> a -> a
  lift2 f (toWord -> a) (toWord -> b) = fromWord (a `f` b)
  {-# MINIMAL toWord, fromWord #-}


type family WordTypeForBits a where
  WordTypeForBits 8 = Data.Word.Word8
  WordTypeForBits 16 = Data.Word.Word16
  WordTypeForBits 32 = Data.Word.Word32
  WordTypeForBits 64 = Data.Word.Word64

instance (Integral (WordTypeForBits n), FiniteBits (WordTypeForBits n), KnownNat n) => WordView (BitLimited n Natural) where
  type WordType (BitLimited n Natural) = WordTypeForBits n
  toWord (NatN n) = fromIntegral n
  fromWord = NatN . fromIntegral

type family IntTypeForBits a where
  IntTypeForBits 8 = Data.Int.Int8
  IntTypeForBits 16 = Data.Int.Int16
  IntTypeForBits 32 = Data.Int.Int32
  IntTypeForBits 64 = Data.Int.Int64

instance (Integral (IntTypeForBits n), FiniteBits (IntTypeForBits n), KnownNat n) => WordView (BitLimited n Integer) where
  type WordType (BitLimited n Integer) = IntTypeForBits n
  toWord (IntN n) = fromIntegral n
  fromWord = IntN . fromIntegral

type WordLike n = ( Integral (WordTypeForBits n)
                  , FiniteBits (WordTypeForBits n)
                  , Integral (IntTypeForBits n)
                  , FiniteBits (IntTypeForBits n)
                  , KnownNat n
                  , WordView (BitLimited n Natural)
                  , WordView (BitLimited n Integer)
                  )

type Nat8 = BitLimited 8 Natural
type Nat16 = BitLimited 16 Natural
type Nat32 = BitLimited 32 Natural
type Nat64 = BitLimited 64 Natural
type Int8 = BitLimited 8 Integer
type Int16 = BitLimited 16 Integer
type Int32 = BitLimited 32 Integer
type Int64 = BitLimited 64 Integer

instance ToBitLimited bits a => Num (BitLimited bits a) where
  NatN m + NatN n = NatN . fromJust $ trapNat (natVal (Proxy @bits)) (toInteger m + toInteger n)
  IntN m + IntN n = IntN . fromJust $ trapInt (natVal (Proxy @bits)) (m + n)
  NatN m - NatN n = NatN . fromJust $ trapNat (natVal (Proxy @bits)) (toInteger m - toInteger n)
  IntN m - IntN n = IntN . fromJust $ trapInt (natVal (Proxy @bits)) (m - n)
  NatN m * NatN n = NatN . fromJust $ trapNat (natVal (Proxy @bits)) (toInteger m * toInteger n)
  IntN m * IntN n = IntN . fromJust $ trapInt (natVal (Proxy @bits)) (m * n)

  abs (NatN m) = NatN (abs m)
  abs (IntN m) = IntN . fromJust $ trapInt (natVal (Proxy @bits)) (abs m)

  signum (NatN m) = NatN (signum m)
  signum (IntN m) = IntN (signum m)

  fromInteger = toBitLimited . fromInteger

-- Helper class ToBitLimited, to build correct BitLimited variant
class KnownNat bits => ToBitLimited bits a where toBitLimited :: Integer -> BitLimited bits a

instance KnownNat bits => ToBitLimited bits Natural where
  toBitLimited = NatN @bits . fromJust . trapNat (natVal (Proxy @bits)) . fromIntegral
instance KnownNat bits => ToBitLimited bits Integer where
  toBitLimited = IntN @bits . fromJust . trapInt (natVal (Proxy @bits))

instance ToBitLimited n a => Real (BitLimited n a)
instance Enum (BitLimited n a)
instance ToBitLimited n a => Integral (BitLimited n a) where
  toInteger (NatN n) = toInteger n
  toInteger (IntN n) = n

trapNat :: Integer -> Integer -> Maybe Natural
trapNat n v = do guard (v >= 0 && v < 2 ^ n); pure (fromIntegral v)

trapInt :: Integer -> Integer -> Maybe Integer
trapInt n v = do guard (v < 2 ^ (n-1) && v >= - 2 ^ (n-1)); pure v

instance WordLike bits => Evaluatable (BitLimited bits Natural) where
  evaluate Five = pure $ NatN 5
  evaluate (Lit n) = NatN <$> trapNat (natVal (Proxy @bits)) n
  evaluate (Neuralgic n) = NatN <$> trapNat (natVal (Proxy @bits)) (evalN n)
  evaluate ab =
      case ab of
        a `Add` b -> go (+) a b
        a `Sub` b -> go (-) a b
        a `Mul` b -> go (*) a b
        _ `Div` (evaluate -> Just 0) -> Nothing
        a `Div` b -> go quot a b
        _ `Mod` (evaluate -> Just 0) -> Nothing
        a `Mod` b -> go rem a b
        a `Pow` b -> do b' <- evaluate b; exponentiable b'; go (^) a b
          -- NB: The use of exponentiable means that evaluate t = Nothing
          -- can mean "traps" or "too big, should not evaluate"
          -- This is kinda ok, because the generators only produce small
          -- exponents, but could be revisited for better test coverage.
        IfThenElse a b c -> do c <- evaluate c
                               evaluate $ if c then a else b

        a `Or` b -> goWrap (.|.) a b
        a `And` b -> goWrap (.&.) a b
        a `Xor` b -> goWrap xor a b
        a `RotL` b -> goWrapMod rotateL a b
        a `RotR` b -> goWrapMod rotateR a b
        a `ShiftL` b -> goWrapMod shiftL a b
        a `ShiftR` b -> goWrapMod shiftR a b

        Complement a -> fromWord . complement . toWord <$> evaluate a
        PopCnt a -> NatN . fromIntegral . popCount . toWord <$> evaluate a
        Clz a -> NatN . fromIntegral . countLeadingZeros . toWord <$> evaluate a
        Ctz a -> NatN . fromIntegral . countTrailingZeros . toWord <$> evaluate a

        a `WrapAdd` b -> goWrap (+) a b
        a `WrapSub` b -> goWrap (-) a b
        a `WrapMul` b -> goWrap (*) a b
        a `WrapPow` b -> goWrap (^) a b

        _ -> error $ show ab
    where
        bitcount = natVal (Proxy @bits)
        go op a b = do NatN a <- evaluate a; NatN b <- evaluate b; NatN <$> trapNat (natVal (Proxy @bits)) (toInteger a `op` toInteger b)
        goWrap op a b = do
            a <- toWord <$> evaluate a
            b <- toWord <$> evaluate b
            pure $ fromWord $ a `op` b
        goWrapMod op a b = do
            a <- toWord <$> evaluate a
            b <- fromIntegral . (`mod` bitcount) . fromIntegral <$> evaluate b
            pure $ fromWord $ a `op` b

instance WordLike bits => Evaluatable (BitLimited bits Integer) where
  evaluate Five = pure $ IntN 5
  evaluate (Lit n) = IntN <$> trapInt (natVal (Proxy @bits)) n
  evaluate (Neuralgic n) = IntN <$> trapInt (natVal (Proxy @bits)) (evalN n)
  evaluate ab =
      case ab of
        a `Add` b -> go (+) a b
        a `Sub` b -> go (-) a b
        a `Mul` b -> go (*) a b
        _ `Div` (evaluate -> Just 0) -> Nothing
        a `Div` b -> go quot a b
        _ `Mod` (evaluate -> Just 0) -> Nothing
        a `Mod` b -> go rem a b
        a `Pow` b -> do b' <- evaluate b; exponentiable b'; go (^) a b
        IfThenElse a b c -> do c <- evaluate c
                               evaluate $ if c then a else b

        a `Or` b -> goWrap (.|.) a b
        a `And` b -> goWrap (.&.) a b
        a `Xor` b -> goWrap xor a b
        a `RotL` b -> goWrapMod rotateL a b
        a `RotR` b -> goWrapMod rotateR a b
        a `ShiftL` b -> goWrapMod shiftL a b
        a `ShiftR` b -> goWrapMod shiftR a b

        Complement a -> fromWord . complement . toWord <$> evaluate a
        PopCnt a -> IntN . fromIntegral . popCount . toWord <$> evaluate a
        Clz a -> IntN . fromIntegral . countLeadingZeros . toWord <$> evaluate a
        Ctz a -> IntN . fromIntegral . countTrailingZeros . toWord <$> evaluate a

        a `WrapAdd` b -> goWrap (+) a b
        a `WrapSub` b -> goWrap (-) a b
        a `WrapMul` b -> goWrap (*) a b
        a `WrapPow` b -> do b' <- evaluate b; exponentiable b'; goWrap (^) a b

        _ -> error $ show ab
    where
        bitcount = natVal (Proxy @bits)
        go op a b = do IntN a <- evaluate a; IntN b <- evaluate b; IntN <$> trapInt (natVal (Proxy @bits)) (toInteger a `op` toInteger b)
        goWrap op a b = do
            a <- toWord <$> evaluate a
            b <- toWord <$> evaluate b
            pure $ fromWord $ a `op` b
        goWrapMod op a b = do
            a <- toWord <$> evaluate a
            b <- fromIntegral . (`mod` bitcount) . fromIntegral <$> evaluate b
            pure $ fromWord $ a `op` b

instance Evaluatable Integer where
  evaluate = eval
instance Evaluatable Natural where
  evaluate = eval

instance (Evaluatable a, Evaluatable b) => Evaluatable (a, b) where
  evaluate (Pair a b) = (,) <$> evaluate a <*> evaluate b

instance (Evaluatable a, Evaluatable b, Evaluatable c) => Evaluatable (a, b, c) where
  evaluate (Triple a b c) = (,,) <$> evaluate a <*> evaluate b <*> evaluate c

instance Evaluatable a => Evaluatable (Maybe a) where
  evaluate Null = pure Nothing
  evaluate (Some a) = Just <$> evaluate a

maskFor :: forall n a . (WordLike n, Num a) => MOTerm (BitLimited n a) -> a
maskFor _ = fromIntegral $ 2 ^ natVal (Proxy @n) - 1


eval :: (Restricted a, Integral a) => MOTerm a -> Maybe a
eval Five = pure 5
eval (Neuralgic n) = evalN n
eval (Lit n) = pure (fromIntegral n)
eval (Pos n) = eval n
eval (Neg n) = - eval n
eval (Abs n) = abs $ eval n
eval (a `Add` b) = eval a + eval b
eval ((eval -> a) `Sub` (eval -> b)) = do guard $ substractable a b; a - b
eval (a `Mul` b) = eval a * eval b
eval (a `Div` b) = eval a `quot` eval b
eval (a `Mod` b) = eval a `rem` eval b
eval (a `Pow` (eval -> b)) = do b' <- b; exponentiable b'; (^) <$> eval a <*> b
eval (ConvertNatural t) = fromIntegral <$> evaluate t
eval c@(ConvertNatToNatN t) = fromIntegral . (.&. maskFor c) <$> evaluate t
eval (ConvertNatNToNat t) = fromIntegral <$> evaluate t
eval c@(ConvertIntToIntN t) = fromIntegral . (.&. maskFor c) <$> evaluate t
eval (ConvertIntNToInt t) = fromIntegral <$> evaluate t
eval (IfThenElse a b c) = do c <- evaluate c
                             eval $ if c then a else b
--eval _ = Nothing


instance Evaluatable Bool where
  evaluate (a `NotEqual` b) = (/=) <$> evaluate a <*> evaluate b
  evaluate (a `Equals` b) = (==) <$> evaluate a <*> evaluate b
  evaluate (a `GreaterEqual` b) = (>=) <$> evaluate a <*> evaluate b
  evaluate (a `Greater` b) = (>) <$> evaluate a <*> evaluate b
  evaluate (a `LessEqual` b) = (<=) <$> evaluate a <*> evaluate b
  evaluate (a `Less` b) = (<) <$> evaluate a <*> evaluate b
  evaluate (a `ShortAnd` b) = evaluate a >>= bool (pure False) (evaluate b)
  evaluate (a `ShortOr` b) = evaluate a >>= bool (evaluate b) (pure True)
  evaluate (Not a) = not <$> evaluate a
  evaluate (Bool b) = pure b

instance Evaluatable String where
  evaluate (Text a) = pure a
  evaluate (a `Concat` b) = (<>) <$> evaluate a <*> evaluate b

type AnnotLit t = (Annot t, Literal t)

class Annot t where
  annot :: MOTerm t -> String -> String
  sizeSuffix :: MOTerm t -> String -> String
  sizeSuffix _ = id
  typSuffix :: MOTerm t -> String -> String
  typSuffix _ = id

instance Annot (a, b) where
  annot _ = id

instance Annot (a, b, c) where
  annot _ = id

instance Annot (Maybe a) where
  annot _ = id

instance Annot Integer where
  annot _ s = "((" <> s <> ") : Int)"

instance Annot Natural where
  annot _ s = "((" <> s <> ") : Nat)"

instance Annot String where
  annot _ = id

bitWidth :: KnownNat n => Proxy n -> String
bitWidth p = show (natVal p)

instance KnownNat n => Annot (BitLimited n Natural) where
  annot _ s = "((" <> s <> ") : Nat" <> bitWidth (Proxy @n) <> ")"
  sizeSuffix _ = (<> bitWidth (Proxy @n))
  typSuffix _ = (<> "Nat" <> bitWidth (Proxy @n))

instance KnownNat n => Annot (BitLimited n Integer) where
  annot _ s = "((" <> s <> ") : Int" <> bitWidth (Proxy @n) <> ")"
  sizeSuffix _ = (<> bitWidth (Proxy @n))
  typSuffix _ = (<> "Int" <> bitWidth (Proxy @n))

instance Annot Bool where
  annot _ = id

class Literal a where
  literal :: Neuralgic a -> String
  literal (evalN -> n) = if n < 0 then "(" <> show n <> ")" else show n

instance Literal (a, b) where
  literal _ = error "Literal (a, b) makes no sense"

instance Literal (a, b, c) where
  literal _ = error "Literal (a, b, c) makes no sense"

instance Literal (Maybe a) where
  literal _ = error "Literal (Maybe a) makes no sense"

instance Literal String where
  literal _ = error "Literal String makes no sense"

instance Literal Integer
instance Literal Natural
instance Literal (BitLimited n Natural)
instance KnownNat bits => Literal (BitLimited bits Integer) where
  literal (evalN -> n) = if n < 0
                         then "(" <> show n <> ")"
                         else show n

instance Literal Bool where
  literal _ = error "Literal Bool makes no sense"

inParens :: (a -> String) -> String -> a -> a -> String
inParens to op lhs rhs = "(" <> to lhs <> " " <> op <> " " <> to rhs <> ")"

unparseMO :: AnnotLit a => MOTerm a -> String
unparseMO f@Five = annot f "5"
unparseMO a@(Lit n) = annot a $ show n
unparseMO a@(Neuralgic n) = annot a $ literal n
unparseMO (Pos n) = "(+" <> unparseMO n <> ")"
unparseMO (Neg n) = "(-" <> unparseMO n <> ")"
unparseMO (Abs n) = "(Prim.abs " <> unparseMO n <> ")"
unparseMO (a `Add` b) = inParens unparseMO "+" a b
unparseMO (a `Sub` b) = annot a $ inParens unparseMO "-" a b
unparseMO (a `Mul` b) = inParens unparseMO "*" a b
unparseMO (a `Div` b) = inParens unparseMO "/" a b
unparseMO (a `Mod` b) = inParens unparseMO "%" a b
unparseMO (a `Pow` b) = inParens unparseMO "**" a b
unparseMO (a `Or` b) = inParens unparseMO "|" a b
unparseMO (a `And` b) = inParens unparseMO "&" a b
unparseMO (a `Xor` b) = inParens unparseMO "^" a b
unparseMO (a `RotL` b) = inParens unparseMO "<<>" a b
unparseMO (a `RotR` b) = inParens unparseMO "<>>" a b
unparseMO (a `ShiftL` b) = inParens unparseMO "<<" a b
unparseMO (a `ShiftR` b) = inParens unparseMO ">>" a b
unparseMO (a `WrapAdd` b) = inParens unparseMO "+%" a b
unparseMO (a `WrapSub` b) = inParens unparseMO "-%" a b
unparseMO (a `WrapMul` b) = inParens unparseMO "*%" a b
unparseMO (a `WrapPow` b) = inParens unparseMO "**%" a b
unparseMO t@(PopCnt n) = typSuffix t "(Prim.popcnt" <> " " <> unparseMO n <> ")"
unparseMO t@(Clz n) = typSuffix t "(Prim.clz" <> " " <> unparseMO n <> ")"
unparseMO t@(Ctz n) = typSuffix t "(Prim.ctz" <> " " <> unparseMO n <> ")"
unparseMO (Complement a) = "(^ " <> unparseMO a <> ")"
unparseMO (ConvertNatural a) = "(++++(" <> unparseMO a <> "))"
unparseMO (ConvertNatNToNat a) = sizeSuffix a "(Prim.nat" <> "ToNat " <> unparseMO a <> ")"
unparseMO t@(ConvertNatToNatN a) = sizeSuffix t "(Prim.intToNat" <> "Wrap " <> unparseMO a <> ")"
unparseMO (ConvertIntNToInt a) = sizeSuffix a "(Prim.int" <> "ToInt " <> unparseMO a <> ")"
unparseMO t@(ConvertIntToIntN a) = sizeSuffix t "(Prim.intToInt" <> "Wrap " <> unparseMO a <> ")"
unparseMO (IfThenElse a b c) = "(if (" <> unparseMO c <> ") " <> unparseMO a <> " else " <> unparseMO b <> ")"
unparseMO (a `NotEqual` b) = inParens unparseMO "!=" a b
unparseMO (a `Equals` b) = inParens unparseMO "==" a b
unparseMO (a `GreaterEqual` b) = inParens unparseMO ">=" a b
unparseMO (a `Greater` b) = inParens unparseMO ">" a b
unparseMO (a `LessEqual` b) = inParens unparseMO "<=" a b
unparseMO (a `Less` b) = inParens unparseMO "<" a b
unparseMO (a `ShortAnd` b) = inParens unparseMO "and" a b
unparseMO (a `ShortOr` b) = inParens unparseMO "or" a b
unparseMO (Not a) = "(not " <> unparseMO a <> ")"
unparseMO (Bool False) = "false"
unparseMO (Bool True) = "true"
unparseMO (a `Pair` b) = "(" <> unparseMO a <> ", " <> unparseMO b <> ")"
unparseMO (Triple a b c) = "(" <> unparseMO a <> ", " <> unparseMO b <> ", " <> unparseMO c <> ")"
unparseMO Null = "null"
unparseMO (Some a) = '?' : unparseMO a
unparseMO (Text a) = '"' : concatMap escape a <> "\""
unparseMO (a `Concat` b) = "(" <> unparseMO a <> " # " <> unparseMO b <> ")"
unparseMO (Array _ ) = error "not yet implemented"


-- TODOs:
--   - wordToInt
--   - natToNat64/intToWord64 (and round-trips)
--       Done: natToWordN/wordNToNat
--   - bitwise ops (btst?)
--   - pattern matches (over numeric, bool, structured)
--   - trapping flavour-preserving conversions Nat -> NatN
--   - bitsize-preserving conversions
--   - data MOTerm (p :: {Term, Pattern}) where ... Pattern :: MOValue a => a -> MOTerm (Pattern/both) a
--   - Text and Concat tests (partly covered by rope tests)
