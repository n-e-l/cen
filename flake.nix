{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable."1.92.0".default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };
        swiftshader = pkgs.swiftshader.overrideAttrs (old: {
          postPatch = (old.postPatch or "") + ''
            sed -i '1s/^/#include <cstdint>\n/' \
              third_party/glslang/SPIRV/SpvBuilder.h
          '';
        });
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            
            # Development tools
            rust-analyzer
            cargo-watch
            cargo-edit

			# GPU
			vulkan-loader
			vulkan-headers
			vulkan-tools
			vulkan-validation-layers
			glslang
			spirv-tools
			shaderc
            mesa # for lavapipe
			shader-slang  

			openssl
			pkg-config

			# Wayland
			wayland
			wayland-protocols
            libxkbcommon
          ];

          # Set library paths
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.vulkan-loader
            pkgs.shaderc
            pkgs.shader-slang
          ];

		  shellHook = ''
				export VULKAN_SDK="${pkgs.vulkan-loader}"
				export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
				export LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:${pkgs.vulkan-loader}/lib:$LD_LIBRARY_PATH"
                export SHADERC_LIB_DIR="${pkgs.shaderc.lib}/lib"
				export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
				export SLANG_LIB_DIR="${pkgs.shader-slang}/lib"
				export SLANG_INCLUDE_DIR="${pkgs.shader-slang.dev}/include"
				export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include"
          '';

          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

        };
      });
}
