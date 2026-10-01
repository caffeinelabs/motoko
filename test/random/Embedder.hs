{-# language OverloadedStrings, {-PartialTypeSignatures,-} ScopedTypeVariables, TupleSections #-}

module Embedder (Embedder(..), WasmAPI(..), embedder, isHealthy, embedderCommand, addCompilerArgs, addEmbedderArgs, invokeEmbedder) where

import Prelude hiding (unwords)

import Test.QuickCheck

import Turtle
import Turtle.Pipe

import Control.Monad.Catch
import GHC.IO.Exception (IOException)
import Data.Text (unwords)
import Data.IORef
-- import Debug.Trace (traceShowId, traceShow)

data WasmAPI = DontPrint | WASI

data Embedder = WasmTime WasmAPI | Drun

instance Arbitrary Embedder where arbitrary = elements [WasmTime DontPrint, WasmTime WASI, Drun]
instance Arbitrary WasmAPI where arbitrary = elements [DontPrint, WASI]

embedderCommand (WasmTime _) = "wasmtime"
embedderCommand Drun = "drun"

isHealthy :: Embedder -> IO Bool
isHealthy e = do
  (res, _) <- shellStrict (embedderCommand e <> " --help") empty
  pure $ res == ExitSuccess

addCompilerArgs (WasmTime _) = ("-wasi-system-api" :)
addCompilerArgs Drun = id

addEmbedderArgs (WasmTime _) = \args -> ["-C", "cache=n", "-W", "nan-canonicalization=y", "-W", "memory64", "-W", "multi-memory", "-W", "bulk-memory"] <> args
addEmbedderArgs Drun = ("--extra-batches" :) . ("10" :)

embedderInvocation :: Embedder -> [Text] -> [Text]
embedderInvocation e args = embedderCommand e : addEmbedderArgs e args

embedderMassageResult (WasmTime DontPrint) (stat, _, stderr) = (stat, "", stderr)
embedderMassageResult (WasmTime _) res = res
embedderMassageResult Drun res = res

invokeEmbedder :: Embedder -> Turtle.FilePath -> IO (ExitCode, Text, Text)
invokeEmbedder embedder wasm = go embedder
  where fileArg = fromString . encodeString
        Right wasmFile = toText wasm
        revconcating = Fold (flip (:)) [] id
        go :: Embedder -> IO (ExitCode, Text, Text)
        go Drun = do
          fuzz <- newIORef ""

          sh $ do
            let Right w = toText wasm
                control = wasm <.> "fifo"
            rm (fileArg control) `catch` \(_ :: GHC.IO.Exception.IOException) -> pure () -- rm -f
            let Right c = toText control
            procs "mkfifo" [c] empty
            consumer <- forkShell $ inshell (unwords $ embedderInvocation embedder [c]) empty
            let create = unsafeTextToLine $ format "create"
            let install = unsafeTextToLine $ format ("install rwlgt-iiaaa-aaaaa-aaaaa-cai "%s%" 0x") w

            pipe (fileArg control) (pure create <|> pure install
                                   <|> "ingress rwlgt-iiaaa-aaaaa-aaaaa-cai go 0x4449444c0000")
            lns <- wait consumer
            -- view lns
            let errors = grep (has "Err: " <|> has "Reject: ") lns
            linesToText . reverse <$> fold errors revconcating >>= liftIO <$> writeIORef fuzz

          (ExitSuccess, "",) <$> readIORef fuzz

        go _ = embedderMassageResult embedder
               <$> procStrictWithErr (embedderCommand embedder) (addEmbedderArgs embedder [wasmFile]) empty
        -- forkShell :: Show a => Shell a -> Shell (Async (Shell a))
        forkShell shell = do a <- fork (fold shell revconcating)
                             pure (select . reverse <$> a)


embedder :: Embedder
embedder = WasmTime DontPrint
