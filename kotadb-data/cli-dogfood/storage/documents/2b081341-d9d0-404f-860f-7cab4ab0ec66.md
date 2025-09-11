---
tags:
- file
- kota-db
- ext_xml
---
<?xml version="1.0" encoding="UTF-8"?>
<bom xmlns="http://cyclonedx.org/schema/bom/1.3" serialNumber="urn:uuid:1b6d677e-7f87-4117-ba0e-b9a26760e2af" version="1">
  <metadata>
    <timestamp>2025-08-07T14:25:28.777804000Z</timestamp>
    <tools>
      <tool>
        <vendor>CycloneDX</vendor>
        <name>cargo-cyclonedx</name>
        <version>0.5.7</version>
      </tool>
    </tools>
    <authors>
      <author>
        <name>KotaDB Contributors</name>
      </author>
    </authors>
    <component type="application" bom-ref="path+file:///Users/jayminwest/Projects/kota-db#kotadb@0.1.0">
      <author>KotaDB Contributors</author>
      <name>kotadb</name>
      <version>0.1.0</version>
      <description>A custom database for distributed human-AI cognition</description>
      <scope>required</scope>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/kotadb@0.1.0?download_url=file://.</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/jayminwest/kota-db</url>
        </reference>
      </externalReferences>
      <components>
        <component type="library" bom-ref="path+file:///Users/jayminwest/Projects/kota-db#kotadb@0.1.0 bin-target-0">
          <name>kotadb</name>
          <version>0.1.0</version>
          <purl>pkg:cargo/kotadb@0.1.0?download_url=file://.#src/lib.rs</purl>
        </component>
        <component type="application" bom-ref="path+file:///Users/jayminwest/Projects/kota-db#kotadb@0.1.0 bin-target-1">
          <name>kotadb</name>
          <version>0.1.0</version>
          <purl>pkg:cargo/kotadb@0.1.0?download_url=file://.#src/main.rs</purl>
        </component>
      </components>
    </component>
    <properties>
      <property name="cdx:rustc:sbom:target:triple">aarch64-apple-darwin</property>
    </properties>
  </metadata>
  <components>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#aho-corasick@1.1.3">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>aho-corasick</name>
      <version>1.1.3</version>
      <description>Fast multiple substring searching.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8e60d3430d3a69478ad0993f19238d2df97c507009a52b3c10addcd7f6bcb916</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/aho-corasick@1.1.3</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/BurntSushi/aho-corasick</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/aho-corasick</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#anstream@0.6.20">
      <name>anstream</name>
      <version>0.6.20</version>
      <description>IO stream adapters for writing colored text that will gracefully degrade according to your terminal's capabilities.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3ae563653d1938f79b1ab1b5e668c87c76a9930414574a6583a7b7e11a8e6192</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/anstream@0.6.20</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-cli/anstyle.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-parse@0.2.7">
      <name>anstyle-parse</name>
      <version>0.2.7</version>
      <description>Parse ANSI Style Escapes</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">4e7644824f0aa2c7b9384579234ef10eb7efb6a0deb83f9630a49594dd9c15c2</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/anstyle-parse@0.2.7</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-cli/anstyle.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-query@1.1.4">
      <name>anstyle-query</name>
      <version>1.1.4</version>
      <description>Look up colored console capabilities</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">9e231f6134f61b71076a3eab506c379d4f36122f2af15a9ff04415ea4c3339e2</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/anstyle-query@1.1.4</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-cli/anstyle.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#anstyle@1.0.11">
      <name>anstyle</name>
      <version>1.0.11</version>
      <description>ANSI text styling</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">862ed96ca487e809f1c8e5a8447f6ee2cf102f846893800b20cebdf541fc6bbd</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/anstyle@1.0.11</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-cli/anstyle.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#anyhow@1.0.98">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>anyhow</name>
      <version>1.0.98</version>
      <description>Flexible concrete Error type built on std::error::Error</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e16d2d3311acee920a9eb8d33b8cbc1787ce4a264e85f964c2404b969bdcd487</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/anyhow@1.0.98</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/anyhow</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/anyhow</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#async-trait@0.1.88">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>async-trait</name>
      <version>0.1.88</version>
      <description>Type erasure for async trait methods</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e539d3fca749fcee5236ab05e93a52867dd549cc157c8cb7f99595f3cedffdb5</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/async-trait@0.1.88</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/async-trait</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/async-trait</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#autocfg@1.5.0">
      <author>Josh Stone &lt;cuviper@gmail.com&gt;</author>
      <name>autocfg</name>
      <version>1.5.0</version>
      <description>Automatic cfg for Rust compiler features</description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">c08606f8c3cbf4ce6ec8e28fb0014a2c086708fe954eaa885384a6165172e7e8</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/autocfg@1.5.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/autocfg/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/cuviper/autocfg</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#axum-core@0.4.5">
      <name>axum-core</name>
      <version>0.4.5</version>
      <description>Core types and traits for axum</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">09f2bd6146b97ae3359fa0cc6d6b376d9539582c7b4220f041a33ec24c226199</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/axum-core@0.4.5</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/tokio-rs/axum</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/axum</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#axum@0.7.9">
      <name>axum</name>
      <version>0.7.9</version>
      <description>Web framework that focuses on ergonomics and modularity</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">edca88bc138befd0323b20752846e6587272d3b03b0343c8ea28a6f819e6e71f</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/axum@0.7.9</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/tokio-rs/axum</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/axum</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#bincode@1.3.3">
      <author>Ty Overby &lt;ty@pre-alpha.com&gt;, Francesco Mazzoli &lt;f@mazzo.li&gt;, David Tolnay &lt;dtolnay@gmail.com&gt;, Zoey Riordan &lt;zoey@dos.cafe&gt;</author>
      <name>bincode</name>
      <version>1.3.3</version>
      <description>A binary serialization / deserialization strategy that uses Serde for transforming structs into bytes and vice versa!</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b1f45e9417d87227c7a56d22e471c6206462cba514c7590c09aff4cf6d1ddcad</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/bincode@1.3.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/bincode</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/servo/bincode</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#bitflags@2.9.1">
      <author>The Rust Project Developers</author>
      <name>bitflags</name>
      <version>2.9.1</version>
      <description>A macro to generate structures which behave like bitflags. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1b8e56985ec62d17e9c1001dc89c88ecd7dc08e47eba5ec7c29c7b5eeecde967</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/bitflags@2.9.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/bitflags</url>
        </reference>
        <reference type="website">
          <url>https://github.com/bitflags/bitflags</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/bitflags/bitflags</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#block-buffer@0.10.4">
      <author>RustCrypto Developers</author>
      <name>block-buffer</name>
      <version>0.10.4</version>
      <description>Buffer type for block processing of data</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/block-buffer@0.10.4</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/block-buffer</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RustCrypto/utils</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#bytemuck@1.23.1">
      <author>Lokathor &lt;zefria@gmail.com&gt;</author>
      <name>bytemuck</name>
      <version>1.23.1</version>
      <description>A crate for mucking around with piles of bytes.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5c76a5792e44e4abe34d3abf15636779261d45a7450612059293d1d2cfc63422</hash>
      </hashes>
      <licenses>
        <expression>Zlib OR Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/bytemuck@1.23.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/Lokathor/bytemuck</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#byteorder@1.5.0">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>byteorder</name>
      <version>1.5.0</version>
      <description>Library for reading/writing numbers in big-endian and little-endian.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1fd0f2584146f6f2ef48085050886acf353beff7305ebd1ae69500e27c67f64b</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/byteorder@1.5.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/byteorder</url>
        </reference>
        <reference type="website">
          <url>https://github.com/BurntSushi/byteorder</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/byteorder</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1">
      <author>Carl Lerche &lt;me@carllerche.com&gt;, Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>bytes</name>
      <version>1.10.1</version>
      <description>Types and traits for working with bytes</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">d71b6127be86fdcfddb610f7182ac57211d4b18a3e9c82eb2d17662f2227ad6a</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/bytes@1.10.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/bytes</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#cc@1.2.31">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>cc</name>
      <version>1.2.31</version>
      <description>A build-time dependency for Cargo build scripts to assist in invoking the native C compiler to compile native C code into a static archive to be linked into Rust code. </description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">c3a42d84bb6b69d3a8b3eaacf0d88f179e1929695e1ad012b6cf64d9caaa5fd2</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/cc@1.2.31</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/cc</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-lang/cc-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/cc-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>cfg-if</name>
      <version>1.0.1</version>
      <description>A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like an if-else chain, the first matching branch is the item that gets emitted. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">9555578bc9e57714c812a1f84e4fc5b4d21fcb063490c624de019f7464c91268</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/cfg-if@1.0.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-lang/cfg-if</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#chrono@0.4.41">
      <name>chrono</name>
      <version>0.4.41</version>
      <description>Date and time library for Rust</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">c469d952047f47f91b68d1cba3f10d63c11d73e4636f24f08daf0278abf01c4d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/chrono@0.4.41</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/chrono/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/chronotope/chrono</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/chronotope/chrono</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#clap@4.5.42">
      <name>clap</name>
      <version>4.5.42</version>
      <description>A simple to use, efficient, and full-featured Command Line Argument Parser</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">ed87a9d530bb41a67537289bafcac159cb3ee28460e0a4571123d2a778a6a882</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/clap@4.5.42</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/clap-rs/clap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#clap_builder@4.5.42">
      <name>clap_builder</name>
      <version>4.5.42</version>
      <description>A simple to use, efficient, and full-featured Command Line Argument Parser</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">64f4f3f3c77c94aff3c7e9aac9a2ca1974a5adf392a8bb751e827d6d127ab966</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/clap_builder@4.5.42</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/clap-rs/clap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#clap_derive@4.5.41">
      <name>clap_derive</name>
      <version>4.5.41</version>
      <description>Parse command line argument by defining a struct, derive crate.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">ef4f52386a59ca4c860f7393bcf8abd8dfd91ecccc0f774635ff68e92eeef491</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/clap_derive@4.5.41</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/clap-rs/clap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#clap_lex@0.7.5">
      <name>clap_lex</name>
      <version>0.7.5</version>
      <description>Minimal, flexible command line parser</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b94f61472cee1439c0b966b47e3aca9ae07e45d070759512cd390ea2bebc6675</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/clap_lex@0.7.5</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/clap-rs/clap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#colorchoice@1.0.4">
      <name>colorchoice</name>
      <version>1.0.4</version>
      <description>Global override of color control</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b05b61dc5112cbb17e4b6cd61790d9845d13888356391624cbe7e41efeac1e75</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/colorchoice@1.0.4</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-cli/anstyle.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#core-foundation-sys@0.8.7">
      <author>The Servo Project Developers</author>
      <name>core-foundation-sys</name>
      <version>0.8.7</version>
      <description>Bindings to Core Foundation for macOS</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">773648b94d0e5d620f64f280777445740e61fe701025087ec8b57f45c791888b</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/core-foundation-sys@0.8.7</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/servo/core-foundation-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/servo/core-foundation-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#cpufeatures@0.2.17">
      <author>RustCrypto Developers</author>
      <name>cpufeatures</name>
      <version>0.2.17</version>
      <description>Lightweight runtime CPU feature detection for aarch64, loongarch64, and x86/x86_64 targets,  with no_std support and support for mobile targets including Android and iOS </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">59ed5838eebb26a2bb2e58f6d5b5316989ae9d08bab10e0e6d103e656d1b0280</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/cpufeatures@0.2.17</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/cpufeatures</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RustCrypto/utils</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#crc32c@0.6.8">
      <author>Zack Owens</author>
      <name>crc32c</name>
      <version>0.6.8</version>
      <description>Safe implementation for hardware accelerated CRC32C instructions with software fallback</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3a47af21622d091a8f0fb295b88bc886ac74efcc613efc19f5d0b21de5c89e47</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/crc32c@0.6.8</purl>
      <externalReferences>
        <reference type="documentation">
          <url>http://docs.rs/crc32c</url>
        </reference>
        <reference type="website">
          <url>https://github.com/zowens/crc32c</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/zowens/crc32c</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-channel@0.5.15">
      <name>crossbeam-channel</name>
      <version>0.5.15</version>
      <description>Multi-producer multi-consumer channels for message passing</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">82b8f8f868b36967f9606790d1903570de9ceaf870a7bf9fbbd3016d636a2cb2</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/crossbeam-channel@0.5.15</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/crossbeam-rs/crossbeam/tree/master/crossbeam-channel</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/crossbeam-rs/crossbeam</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-utils@0.8.21">
      <name>crossbeam-utils</name>
      <version>0.8.21</version>
      <description>Utilities for concurrent programming</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">d0a5c400df2834b80a4c3327b3aad3a4c4cd4de0629063962b03235697506a28</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/crossbeam-utils@0.8.21</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/crossbeam-rs/crossbeam/tree/master/crossbeam-utils</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/crossbeam-rs/crossbeam</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#crypto-common@0.1.6">
      <author>RustCrypto Developers</author>
      <name>crypto-common</name>
      <version>0.1.6</version>
      <description>Common cryptographic traits</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1bfb12502f3fc46cca1bb51ac28df9d618d813cdc3d2f25b9fe775a34af26bb3</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/crypto-common@0.1.6</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/crypto-common</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RustCrypto/traits</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#dashmap@6.1.0">
      <author>Acrimon &lt;joel.wejdenstal@gmail.com&gt;</author>
      <name>dashmap</name>
      <version>6.1.0</version>
      <description>Blazing fast concurrent HashMap for Rust.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5041cc499144891f3790297212f32a74fb938e5136a14943f338ef9e0ae276cf</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/dashmap@6.1.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/dashmap</url>
        </reference>
        <reference type="website">
          <url>https://github.com/xacrimon/dashmap</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/xacrimon/dashmap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#digest@0.10.7">
      <author>RustCrypto Developers</author>
      <name>digest</name>
      <version>0.10.7</version>
      <description>Traits for cryptographic hash functions and message authentication codes</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/digest@0.10.7</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/digest</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RustCrypto/traits</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#equivalent@1.0.2">
      <name>equivalent</name>
      <version>1.0.2</version>
      <description>Traits for key comparison in maps.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">877a4ace8713b0bcf2a4e7eec82529c029f1d0619886d18145fea96c3ffe5c0f</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/equivalent@1.0.2</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/indexmap-rs/equivalent</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#errno@0.3.13">
      <author>Chris Wong &lt;lambda.fairy@gmail.com&gt;, Dan Gohman &lt;dev@sunfishcode.online&gt;</author>
      <name>errno</name>
      <version>0.3.13</version>
      <description>Cross-platform interface to the `errno` variable.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">778e2ac28f6c47af28e4907f13ffd1e1ddbd400980a9abd7c8df189bf578a5ad</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/errno@0.3.13</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/errno</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/lambda-fairy/rust-errno</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#fastrand@2.3.0">
      <author>Stjepan Glavina &lt;stjepang@gmail.com&gt;</author>
      <name>fastrand</name>
      <version>2.3.0</version>
      <description>A simple and fast random number generator</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">37909eebbb50d72f9059c3b6d82c0463f2ff062c9e95845c43a6c9c0355411be</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/fastrand@2.3.0</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/smol-rs/fastrand</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#filetime@0.2.25">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>filetime</name>
      <version>0.2.25</version>
      <description>Platform-agnostic accessors of timestamps in File metadata </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">35c0522e981e68cbfa8c3f978441a5f34b30b96e146b33cd3359176b50fe8586</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/filetime@0.2.25</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/filetime</url>
        </reference>
        <reference type="website">
          <url>https://github.com/alexcrichton/filetime</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/alexcrichton/filetime</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#fnv@1.0.7">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>fnv</name>
      <version>1.0.7</version>
      <description>Fowler–Noll–Vo hash function</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3f9eec918d3f24069decb9af1554cad7c880e2da24a9afd88aca000531ab82c1</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0  OR  MIT</expression>
      </licenses>
      <purl>pkg:cargo/fnv@1.0.7</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://doc.servo.org/fnv/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/servo/rust-fnv</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#form_urlencoded@1.2.1">
      <author>The rust-url developers</author>
      <name>form_urlencoded</name>
      <version>1.2.1</version>
      <description>Parser and serializer for the application/x-www-form-urlencoded syntax, as used by HTML forms.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e13624c2627564efccf4934284bdd98cbaa14e79b0b5a141218e507b3a823456</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/form_urlencoded@1.2.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/servo/rust-url</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#fsevent-sys@4.1.0">
      <author>Pierre Baillet &lt;pierre@baillet.name&gt;</author>
      <name>fsevent-sys</name>
      <version>4.1.0</version>
      <description>Rust bindings to the fsevent macOS API for file changes notifications</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">76ee7a02da4d231650c7cea31349b889be2f45ddb3ef3032d2ec8185f6313fd2</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/fsevent-sys@4.1.0</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/octplane/fsevent-rust/tree/master/fsevent-sys</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-channel@0.3.31">
      <name>futures-channel</name>
      <version>0.3.31</version>
      <description>Channels for asynchronous communication using futures-rs. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">2dff15bf788c671c1934e366d07e30c1814a8ef514e1af724a602e8a2fbe1b10</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-channel@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31">
      <name>futures-core</name>
      <version>0.3.31</version>
      <description>The core traits and types in for the `futures` library. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">05f29059c0c2090612e8d742178b0580d2dc940c837851ad723096f87af6663e</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-core@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-executor@0.3.31">
      <name>futures-executor</name>
      <version>0.3.31</version>
      <description>Executors for asynchronous tasks based on the futures-rs library. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1e28d1d997f585e54aebc3f97d39e72338912123a67330d723fdbb564d646c9f</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-executor@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-io@0.3.31">
      <name>futures-io</name>
      <version>0.3.31</version>
      <description>The `AsyncRead`, `AsyncWrite`, `AsyncSeek`, and `AsyncBufRead` traits for the futures-rs library. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">9e5c1b78ca4aae1ac06c48a526a655760685149f0d465d21f37abfe57ce075c6</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-io@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-macro@0.3.31">
      <name>futures-macro</name>
      <version>0.3.31</version>
      <description>The futures-rs procedural macro implementations. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">162ee34ebcb7c64a8abebc059ce0fee27c2262618d7b60ed8faf72fef13c3650</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-macro@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-sink@0.3.31">
      <name>futures-sink</name>
      <version>0.3.31</version>
      <description>The asynchronous `Sink` trait for the futures-rs library. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e575fab7d1e0dcb8d0c7bcf9a63ee213816ab51902e6d244a95819acacf1d4f7</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-sink@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-task@0.3.31">
      <name>futures-task</name>
      <version>0.3.31</version>
      <description>Tools for working with tasks. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">f90f7dce0722e95104fcb095585910c0977252f286e354b5e3bd38902cd99988</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-task@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31">
      <name>futures-util</name>
      <version>0.3.31</version>
      <description>Common utilities and extension traits for the futures-rs library. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">9fa08315bb612088cc391249efdc3bc77536f16c91f6cf495e6fbe85b20a4a81</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures-util@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#futures@0.3.31">
      <name>futures</name>
      <version>0.3.31</version>
      <description>An implementation of futures and streams featuring zero allocations, composability, and iterator-like interfaces. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">65bc07b1a8bc7c85c5f2e110c476c7389b4554ba72af57d8445ea63a576b0876</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/futures@0.3.31</purl>
      <externalReferences>
        <reference type="website">
          <url>https://rust-lang.github.io/futures-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/futures-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#generic-array@0.14.7">
      <author>Bartłomiej Kamiński &lt;fizyk20@gmail.com&gt;, Aaron Trent &lt;novacrazy@gmail.com&gt;</author>
      <name>generic-array</name>
      <version>0.14.7</version>
      <description>Generic types implementing functionality of arrays</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/generic-array@0.14.7</purl>
      <externalReferences>
        <reference type="documentation">
          <url>http://fizyk20.github.io/generic-array/generic_array/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/fizyk20/generic-array.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.2.16">
      <author>The Rand Project Developers</author>
      <name>getrandom</name>
      <version>0.2.16</version>
      <description>A small cross-platform library for retrieving random data from system source</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">335ff9f135e4384c8150d6f27c6daed433577f86b4750418338c01a1a2528592</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/getrandom@0.2.16</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/getrandom</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-random/getrandom</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.3.3">
      <author>The Rand Project Developers</author>
      <name>getrandom</name>
      <version>0.3.3</version>
      <description>A small cross-platform library for retrieving random data from system source</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">26145e563e54f2cadc477553f1ec5ee650b00862f0a58bcd12cbdc5f0ea2d2f4</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/getrandom@0.3.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/getrandom</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-random/getrandom</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.14.5">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>hashbrown</name>
      <version>0.14.5</version>
      <description>A Rust port of Google's SwissTable hash map</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e5274423e17b7c9fc20b6e7e208532f9b19825d82dfd615708b70edd83df41f1</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/hashbrown@0.14.5</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-lang/hashbrown</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.15.4">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>hashbrown</name>
      <version>0.15.4</version>
      <description>A Rust port of Google's SwissTable hash map</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5971ac85611da7067dbfcabef3c70ebb5606018acd9e2a3903a0da507521e0d5</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/hashbrown@0.15.4</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-lang/hashbrown</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#heck@0.5.0">
      <name>heck</name>
      <version>0.5.0</version>
      <description>heck is a case conversion library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">2304e00983f87ffb38b55b444b5e3b60a884b5d30c0fca7d82fe33449bbe55ea</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/heck@0.5.0</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/withoutboats/heck</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#http-body-util@0.1.3">
      <author>Carl Lerche &lt;me@carllerche.com&gt;, Lucio Franco &lt;luciofranco14@gmail.com&gt;, Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>http-body-util</name>
      <version>0.1.3</version>
      <description>Combinators and adapters for HTTP request or response bodies. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b021d93e26becf5dc7e1b75b1bed1fd93124b374ceb73f43d4d4eafec896a64a</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/http-body-util@0.1.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/http-body-util</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/http-body</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1">
      <author>Carl Lerche &lt;me@carllerche.com&gt;, Lucio Franco &lt;luciofranco14@gmail.com&gt;, Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>http-body</name>
      <version>1.0.1</version>
      <description>Trait representing an asynchronous, streaming, HTTP request or response body. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1efedce1fb8e6913f23e0c92de8e62cd5b772a67e7b3946df930a62566c93184</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/http-body@1.0.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/http-body</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/http-body</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;, Carl Lerche &lt;me@carllerche.com&gt;, Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>http</name>
      <version>1.3.1</version>
      <description>A set of types for representing HTTP requests and responses. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">f4a85d31aea989eead29a3aaf9e1115a180df8282431156e533de47660892565</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/http@1.3.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/http</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/http</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#httparse@1.10.1">
      <author>Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>httparse</name>
      <version>1.10.1</version>
      <description>A tiny, safe, speedy, zero-copy HTTP/1.x parser.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6dbf3de79e51f3d586ab4cb9d5c3e2c14aa28ed23d180cf89b4df0454a69cc87</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/httparse@1.10.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/httparse</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/seanmonstar/httparse</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#httpdate@1.0.3">
      <author>Pyfisch &lt;pyfisch@posteo.org&gt;</author>
      <name>httpdate</name>
      <version>1.0.3</version>
      <description>HTTP date parsing and formatting</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">df3b46402a9d5adb4c86a0cf463f42e19994e3ee891101b1841f30a545cb49a9</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/httpdate@1.0.3</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/pyfisch/httpdate</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#hyper-util@0.1.16">
      <author>Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>hyper-util</name>
      <version>0.1.16</version>
      <description>hyper utilities</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8d9b05277c7e8da2c93a568989bb6207bef0112e8d17df7a6eda4a3cf143bc5e</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/hyper-util@0.1.16</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/hyper-util</url>
        </reference>
        <reference type="website">
          <url>https://hyper.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/hyper-util</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#hyper@1.6.0">
      <author>Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>hyper</name>
      <version>1.6.0</version>
      <description>A protective and efficient HTTP library for all.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">cc2b571658e38e0c01b1fdca3bbbe93c00d3d71693ff2770043f8c29bc7d6f80</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/hyper@1.6.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/hyper</url>
        </reference>
        <reference type="website">
          <url>https://hyper.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/hyper</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#iana-time-zone@0.1.63">
      <author>Andrew Straw &lt;strawman@astraw.com&gt;, René Kijewski &lt;rene.kijewski@fu-berlin.de&gt;, Ryan Lopopolo &lt;rjl@hyperbo.la&gt;</author>
      <name>iana-time-zone</name>
      <version>0.1.63</version>
      <description>get the IANA time zone for the current system</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b0c919e5debc312ad217002b8048a17b7d83f80703865bbfcfebb0458b0b27d8</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/iana-time-zone@0.1.63</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/strawlab/iana-time-zone</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#indexmap@2.10.0">
      <name>indexmap</name>
      <version>2.10.0</version>
      <description>A hash table with consistent order and fast iteration.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">fe4cd85333e22411419a0bcae1297d25e58c9443848b11dc6a86fefe8c78a661</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/indexmap@2.10.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/indexmap/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/indexmap-rs/indexmap</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#is_terminal_polyfill@1.70.1">
      <name>is_terminal_polyfill</name>
      <version>1.70.1</version>
      <description>Polyfill for `is_terminal` stdlib feature for use with older MSRVs</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">7943c866cc5cd64cbc25b2e01621d07fa8eb2a1a23160ee81ce38704e97b8ecf</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/is_terminal_polyfill@1.70.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/polyfill-rs/is_terminal_polyfill</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>itoa</name>
      <version>1.0.15</version>
      <description>Fast integer primitive to string conversion</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">4a5f13b858c8d314ee3e8f639011f7ccefe71f97f96e50151fb991f267928e2c</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/itoa@1.0.15</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/itoa</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/itoa</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#jobserver@0.1.33">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>jobserver</name>
      <version>0.1.33</version>
      <description>An implementation of the GNU Make jobserver for Rust. </description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">38f262f097c174adebe41eb73d66ae9c06b2844fb0da69969647bbddd9b0538a</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/jobserver@0.1.33</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/jobserver</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-lang/jobserver-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/jobserver-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#lazy_static@1.5.0">
      <author>Marvin Löbel &lt;loebel.marvin@gmail.com&gt;</author>
      <name>lazy_static</name>
      <version>1.5.0</version>
      <description>A macro for declaring lazily evaluated statics in Rust.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">bbd2bcb4c963f2ddae06a2efc7e9f3591312473c50c6685e1f298068316e66fe</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/lazy_static@1.5.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/lazy_static</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang-nursery/lazy-static.rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174">
      <author>The Rust Project Developers</author>
      <name>libc</name>
      <version>0.2.174</version>
      <description>Raw FFI bindings to platform libraries like libc.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1171693293099992e19cddea4e8b849964e9846f4acee11b3948bcc337be8776</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/libc@0.2.174</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/rust-lang/libc</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#lock_api@0.4.13">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>lock_api</name>
      <version>0.4.13</version>
      <description>Wrappers to create fully-featured Mutex and RwLock types. Compatible with no_std.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">96936507f153605bddfcda068dd804796c84324ed2510809e5b2a624c81da765</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/lock_api@0.4.13</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/Amanieu/parking_lot</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#log@0.4.27">
      <author>The Rust Project Developers</author>
      <name>log</name>
      <version>0.4.27</version>
      <description>A lightweight logging facade for Rust </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">13dc2df351e3202783a1fe0d44375f7295ffb4049267b0f3018346dc122a1d94</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/log@0.4.27</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/log</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/log</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#lz4-sys@1.11.1+lz4-1.10.0">
      <author>Jens Heyens &lt;jens.heyens@ewetel.net&gt;, Artem V. Navrotskiy &lt;bozaro@buzzsoft.ru&gt;, Patrick Marks &lt;pmarks@gmail.com&gt;</author>
      <name>lz4-sys</name>
      <version>1.11.1+lz4-1.10.0</version>
      <description>Rust LZ4 sys package.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6bd8c0d6c6ed0cd30b3652886bb8711dc4bb01d637a68105a3d5158039b418e6</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/lz4-sys@1.11.1+lz4-1.10.0</purl>
      <externalReferences>
        <reference type="other">
          <url>lz4</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/10xGenomics/lz4-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#lz4@1.28.1">
      <author>Jens Heyens &lt;jens.heyens@ewetel.net&gt;, Artem V. Navrotskiy &lt;bozaro@buzzsoft.ru&gt;, Patrick Marks &lt;pmarks@gmail.com&gt;</author>
      <name>lz4</name>
      <version>1.28.1</version>
      <description>Rust LZ4 bindings library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">a20b523e860d03443e98350ceaac5e71c6ba89aea7d960769ec3ce37f4de5af4</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/lz4@1.28.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/lz4</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/10xGenomics/lz4-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#matchers@0.1.0">
      <author>Eliza Weisman &lt;eliza@buoyant.io&gt;</author>
      <name>matchers</name>
      <version>0.1.0</version>
      <description>Regex matching on character and byte streams. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8263075bb86c5a1b1427b5ae862e8889656f126e9f77c484496e8b47cf5c5558</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/matchers@0.1.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/matchers/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/hawkw/matchers</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hawkw/matchers</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#matchit@0.7.3">
      <author>Ibraheem Ahmed &lt;ibraheem@ibraheem.ca&gt;</author>
      <name>matchit</name>
      <version>0.7.3</version>
      <description>A high performance, zero-copy URL router.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">0e7465ac9959cc2b1404e8e2367b43684a6d13790fe23056cc8c6c5a6b7bcb94</hash>
      </hashes>
      <licenses>
        <expression>MIT AND BSD-3-Clause</expression>
      </licenses>
      <purl>pkg:cargo/matchit@0.7.3</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/ibraheemdev/matchit</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#md5@0.7.0">
      <author>Ivan Ukhov &lt;ivan.ukhov@gmail.com&gt;, Kamal Ahmad &lt;shibe@openmailbox.org&gt;, Konstantin Stepanov &lt;milezv@gmail.com&gt;, Lukas Kalbertodt &lt;lukas.kalbertodt@gmail.com&gt;, Nathan Musoke &lt;nathan.musoke@gmail.com&gt;, Scott Mabin &lt;scott@mabez.dev&gt;, Tony Arcieri &lt;bascule@gmail.com&gt;, Wim de With &lt;register@dewith.io&gt;, Yosef Dinerstein &lt;yosefdi@gmail.com&gt;</author>
      <name>md5</name>
      <version>0.7.0</version>
      <description>The package provides the MD5 hash function.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">490cc448043f947bae3cbee9c203358d62dbee0db12107a74be5c30ccfd09771</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/md5@0.7.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/md5</url>
        </reference>
        <reference type="website">
          <url>https://github.com/stainless-steel/md5</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/stainless-steel/md5</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;, bluss</author>
      <name>memchr</name>
      <version>2.7.5</version>
      <description>Provides extremely fast (uses SIMD on x86_64, aarch64 and wasm32) routines for 1, 2 or 3 byte search and single substring search. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">32a282da65faaf38286cf3be983213fcf1d2e2a58700e808f83f4ea9a4804bc0</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/memchr@2.7.5</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/memchr/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/BurntSushi/memchr</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/memchr</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#memmap2@0.9.7">
      <author>Dan Burkert &lt;dan@danburkert.com&gt;, Yevhenii Reizner &lt;razrfalcon@gmail.com&gt;</author>
      <name>memmap2</name>
      <version>0.9.7</version>
      <description>Cross-platform Rust API for memory-mapped file IO</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">483758ad303d734cec05e5c12b41d7e93e6a6390c5e9dae6bdeb7c1259012d28</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/memmap2@0.9.7</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/memmap2</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RazrFalcon/memmap2-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#mime@0.3.17">
      <author>Sean McArthur &lt;sean@seanmonstar.com&gt;</author>
      <name>mime</name>
      <version>0.3.17</version>
      <description>Strongly Typed Mimes</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6877bb514081ee2a7ff5ef9de3281f14a4dd4bceac4c09388074a6b5df8a139a</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/mime@0.3.17</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/mime</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hyperium/mime</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#mio@1.0.4">
      <author>Carl Lerche &lt;me@carllerche.com&gt;, Thomas de Zeeuw &lt;thomasdezeeuw@gmail.com&gt;, Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>mio</name>
      <version>1.0.4</version>
      <description>Lightweight non-blocking I/O.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">78bed444cc8a2160f01cbcf811ef18cac863ad68ae8ca62092e8db51d51c761c</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/mio@1.0.4</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/tokio-rs/mio</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/mio</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#notify@6.1.1">
      <author>Félix Saparelli &lt;me@passcod.name&gt;, Daniel Faust &lt;hessijames@gmail.com&gt;, Aron Heinecke &lt;Ox0p54r36@t-online.de&gt;</author>
      <name>notify</name>
      <version>6.1.1</version>
      <description>Cross-platform filesystem notification library</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6205bd8bb1e454ad2e27422015fb5e4f2bcc7e08fa8f27058670d208324a4d2d</hash>
      </hashes>
      <licenses>
        <expression>CC0-1.0</expression>
      </licenses>
      <purl>pkg:cargo/notify@6.1.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/notify</url>
        </reference>
        <reference type="website">
          <url>https://github.com/notify-rs/notify</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/notify-rs/notify.git</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#nu-ansi-term@0.46.0">
      <author>ogham@bsago.me, Ryan Scheel (Havvy) &lt;ryan.havvy@gmail.com&gt;, Josh Triplett &lt;josh@joshtriplett.org&gt;, The Nushell Project Developers</author>
      <name>nu-ansi-term</name>
      <version>0.46.0</version>
      <description>Library for ANSI terminal colors and styles (bold, underline)</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">77a8165726e8236064dbb45459242600304b42a5ea24ee2948e18e023bf7ba84</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/nu-ansi-term@0.46.0</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/nushell/nu-ansi-term</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#num-traits@0.2.19">
      <author>The Rust Project Developers</author>
      <name>num-traits</name>
      <version>0.2.19</version>
      <description>Numeric traits for generic mathematics</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">071dfc062690e90b734c0b2273ce72ad0ffa95f0c74596bc250dcfd960262841</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/num-traits@0.2.19</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/num-traits</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-num/num-traits</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-num/num-traits</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3">
      <author>Aleksey Kladov &lt;aleksey.kladov@gmail.com&gt;</author>
      <name>once_cell</name>
      <version>1.21.3</version>
      <description>Single assignment cells and lazy values.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">42f5e15c9953c5e4ccceeb2e7382a716482c34515315f7b03532b8b4e8393d2d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/once_cell@1.21.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/once_cell</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/matklad/once_cell</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#overload@0.1.1">
      <author>Daniel Salvadori &lt;danaugrs@gmail.com&gt;</author>
      <name>overload</name>
      <version>0.1.1</version>
      <description>Provides a macro to simplify operator overloading.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b15813163c1d831bf4a13c3610c05c0d03b39feb07f7e09fa234dac9b15aaf39</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/overload@0.1.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/danaugrs/overload</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot@0.12.4">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>parking_lot</name>
      <version>0.12.4</version>
      <description>More compact and efficient implementations of the standard synchronization primitives.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">70d58bf43669b5795d1576d0641cfb6fbb2057bf629506267a92807158584a13</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/parking_lot@0.12.4</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/Amanieu/parking_lot</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot_core@0.9.11">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>parking_lot_core</name>
      <version>0.9.11</version>
      <description>An advanced API for creating custom synchronization primitives.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">bc838d2a56b5b1a6c25f55575dfc605fabb63bb2365f6c2353ef9159aa69e4a5</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/parking_lot_core@0.9.11</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/Amanieu/parking_lot</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#paste@1.0.15">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>paste</name>
      <version>1.0.15</version>
      <description>Macros for all your token pasting needs</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">57c0d7b74b563b49d38dae00a0c37d4d6de9b432382b2892f0574ddcae73fd0a</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/paste@1.0.15</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/paste</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/paste</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#percent-encoding@2.3.1">
      <author>The rust-url developers</author>
      <name>percent-encoding</name>
      <version>2.3.1</version>
      <description>Percent encoding and decoding</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e3148f5046208a5d56bcfc03053e3ca6334e51da8dfb19b6cdc8b306fae3283e</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/percent-encoding@2.3.1</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/servo/rust-url/</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-internal@1.1.10">
      <name>pin-project-internal</name>
      <version>1.1.10</version>
      <description>Implementation detail of the `pin-project` crate. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6e918e4ff8c4549eb882f14b3a4bc8c8bc93de829416eacf579f1207a8fbf861</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/pin-project-internal@1.1.10</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/taiki-e/pin-project</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16">
      <name>pin-project-lite</name>
      <version>0.2.16</version>
      <description>A lightweight version of pin-project written with declarative macros. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3b3cff922bd51709b605d9ead9aa71031d81447142d828eb4a6eba76fe619f9b</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/pin-project-lite@0.2.16</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/taiki-e/pin-project-lite</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#pin-project@1.1.10">
      <name>pin-project</name>
      <version>1.1.10</version>
      <description>A crate for safe and ergonomic pin-projection. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">677f1add503faace112b9f1373e43e9e054bfdd22ff1a63c1bc485eaec6a6a8a</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/pin-project@1.1.10</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/taiki-e/pin-project</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#pin-utils@0.1.0">
      <author>Josef Brandl &lt;mail@josefbrandl.de&gt;</author>
      <name>pin-utils</name>
      <version>0.1.0</version>
      <description>Utilities for pinning </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8b870d8c151b6f2fb93e84a13146138f05d02ed11c7e7c54f8826aaaf7c9f184</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/pin-utils@0.1.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/pin-utils</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang-nursery/pin-utils</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#pkg-config@0.3.32">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>pkg-config</name>
      <version>0.3.32</version>
      <description>A library to run the pkg-config system tool at build time in order to be used in Cargo build scripts. </description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">7edddbd0b52d732b21ad9a5fab5c704c14cd949e5e9a1ec5929a24fded1b904c</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/pkg-config@0.3.32</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/pkg-config</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/pkg-config-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#ppv-lite86@0.2.21">
      <author>The CryptoCorrosion Contributors</author>
      <name>ppv-lite86</name>
      <version>0.2.21</version>
      <description>Cross-platform cryptography-oriented low-level SIMD library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">85eae3c4ed2f50dcfe72643da4befc30deadb458a9b590d720cde2f2b1e97da9</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/ppv-lite86@0.2.21</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/cryptocorrosion/cryptocorrosion</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;, Alex Crichton &lt;alex@alexcrichton.com&gt;</author>
      <name>proc-macro2</name>
      <version>1.0.95</version>
      <description>A substitute implementation of the compiler's `proc_macro` API to decouple token-based libraries from the procedural macro use case.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">02b3e5e68a3a1a02aad3ec490a98007cbc13c37cbe84a3cd7b8e406d76e7f778</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/proc-macro2@1.0.95</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/proc-macro2</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/proc-macro2</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>quote</name>
      <version>1.0.40</version>
      <description>Quasi-quoting macro quote!(...)</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1885c039570dc00dcb4ff087a89e185fd56bae234ddc7f056a945bf36467248d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/quote@1.0.40</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/quote/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/quote</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rand@0.8.5">
      <author>The Rand Project Developers, The Rust Project Developers</author>
      <name>rand</name>
      <version>0.8.5</version>
      <description>Random number generators and other randomness functionality. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">34af8d1a0e25924bc5b7c43c079c942339d8f0a8b57c39049bef581b46327404</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/rand@0.8.5</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rand</url>
        </reference>
        <reference type="website">
          <url>https://rust-random.github.io/book</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-random/rand</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rand_chacha@0.3.1">
      <author>The Rand Project Developers, The Rust Project Developers, The CryptoCorrosion Contributors</author>
      <name>rand_chacha</name>
      <version>0.3.1</version>
      <description>ChaCha random number generator </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e6c10a63a0fa32252be49d21e7709d4d4baf8d231c2dbce1eaa8141b9b127d88</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/rand_chacha@0.3.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rand_chacha</url>
        </reference>
        <reference type="website">
          <url>https://rust-random.github.io/book</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-random/rand</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rand_core@0.6.4">
      <author>The Rand Project Developers, The Rust Project Developers</author>
      <name>rand_core</name>
      <version>0.6.4</version>
      <description>Core random number generator traits and tools for implementation. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">ec0be4795e2f6a28069bec0b5ff3e2ac9bafc99e6a9a7dc3547996c5c816922c</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/rand_core@0.6.4</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rand_core</url>
        </reference>
        <reference type="website">
          <url>https://rust-random.github.io/book</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-random/rand</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.1.10">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>regex-automata</name>
      <version>0.1.10</version>
      <description>Automata construction and matching using regular expressions.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6c230d73fb8d8c1b9c0b3135c5142a8acee3a0558fb8db5cf1cb65f8d7862132</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/regex-automata@0.1.10</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/regex-automata</url>
        </reference>
        <reference type="website">
          <url>https://github.com/BurntSushi/regex-automata</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/regex-automata</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.4.9">
      <author>The Rust Project Developers, Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>regex-automata</name>
      <version>0.4.9</version>
      <description>Automata construction and matching using regular expressions.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">809e8dc61f6de73b46c85f4c96486310fe304c434cfa43669d7b40f711150908</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/regex-automata@0.4.9</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/regex-automata</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/regex/tree/master/regex-automata</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.6.29">
      <author>The Rust Project Developers</author>
      <name>regex-syntax</name>
      <version>0.6.29</version>
      <description>A regular expression parser.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">f162c6dd7b008981e4d40210aca20b4bd0f9b60ca9271061b07f78537722f2e1</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/regex-syntax@0.6.29</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/regex-syntax</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-lang/regex</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/regex</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.8.5">
      <author>The Rust Project Developers, Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>regex-syntax</name>
      <version>0.8.5</version>
      <description>A regular expression parser.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">2b15c43186be67a4fd63bee50d0303afffcef381492ebe2c5d87f324e1b8815c</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/regex-syntax@0.8.5</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/regex-syntax</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/regex/tree/master/regex-syntax</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#regex@1.11.1">
      <author>The Rust Project Developers, Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>regex</name>
      <version>1.11.1</version>
      <description>An implementation of regular expressions for Rust. This implementation uses finite automata and guarantees linear time matching on all inputs. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b544ef1b4eac5dc2db33ea63606ae9ffcfac26c1416a2806ae0bf5f56b201191</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/regex@1.11.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/regex</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-lang/regex</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/regex</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rmp-serde@1.3.0">
      <author>Evgeny Safronov &lt;division494@gmail.com&gt;</author>
      <name>rmp-serde</name>
      <version>1.3.0</version>
      <description>Serde bindings for RMP</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">52e599a477cf9840e92f2cde9a7189e67b42c57532749bf90aea6ec10facd4db</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/rmp-serde@1.3.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rmp-serde</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/3Hren/msgpack-rust</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rmp@0.8.14">
      <author>Evgeny Safronov &lt;division494@gmail.com&gt;</author>
      <name>rmp</name>
      <version>0.8.14</version>
      <description>Pure Rust MessagePack serialization implementation</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">228ed7c16fa39782c3b3468e974aec2795e9089153cd08ee2e9aefb3613334c4</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/rmp@0.8.14</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rmp</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/3Hren/msgpack-rust</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#roaring@0.10.12">
      <author>Wim Looman &lt;wim@nemo157.com&gt;, Kerollmops &lt;kero@meilisearch.com&gt;</author>
      <name>roaring</name>
      <version>0.10.12</version>
      <description>A better compressed bitset - pure Rust implementation</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">19e8d2cfa184d94d0726d650a9f4a1be7f9b76ac9fdb954219878dc00c1c1e7b</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/roaring@0.10.12</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/roaring</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RoaringBitmap/roaring-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rustc_version@0.4.1">
      <name>rustc_version</name>
      <version>0.4.1</version>
      <description>A library for querying the version of a installed rustc compiler</description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">cfcb3a22ef46e85b45de6ee7e79d063319ebb6594faafcf1c225ea92ab6e9b92</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/rustc_version@0.4.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rustc_version/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/djc/rustc-version-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rustix@1.0.8">
      <author>Dan Gohman &lt;dev@sunfishcode.online&gt;, Jakub Konka &lt;kubkon@jakubkonka.com&gt;</author>
      <name>rustix</name>
      <version>1.0.8</version>
      <description>Safe Rust bindings to POSIX/Unix/Linux/Winsock-like syscalls</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">11181fbabf243db407ef8df94a6ce0b2f9a733bd8be4ad02b4eda9602296cac8</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/rustix@1.0.8</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rustix</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/bytecodealliance/rustix</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#rustversion@1.0.21">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>rustversion</name>
      <version>1.0.21</version>
      <description>Conditional compilation according to rustc compiler version</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8a0d197bd2c9dc6e53b84da9556a69ba4cdfab8619eb41a8bd1cc2027a0f6b1d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/rustversion@1.0.21</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/rustversion</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/rustversion</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#ryu@1.0.20">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>ryu</name>
      <version>1.0.20</version>
      <description>Fast floating point to string conversion</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">28d3b2b1366ec20994f1fd18c3c594f05c5dd4bc44d8bb0c1c632c8d6829481f</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR BSL-1.0</expression>
      </licenses>
      <purl>pkg:cargo/ryu@1.0.20</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/ryu</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/ryu</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#same-file@1.0.6">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>same-file</name>
      <version>1.0.6</version>
      <description>A simple crate for determining whether two file paths point to the same file. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">93fc1dc3aaa9bfed95e02e6eadabb4baf7e3078b0bd1b4d7b6b0b68378900502</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/same-file@1.0.6</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/same-file</url>
        </reference>
        <reference type="website">
          <url>https://github.com/BurntSushi/same-file</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/same-file</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#scopeguard@1.2.0">
      <author>bluss</author>
      <name>scopeguard</name>
      <version>1.2.0</version>
      <description>A RAII scope guard that will run a given closure when it goes out of scope, even if the code between panics (assuming unwinding panic).  Defines the macros `defer!`, `defer_on_unwind!`, `defer_on_success!` as shorthands for guards with one of the implemented strategies. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">94143f37725109f92c262ed2cf5e59bce7498c01bcc1502d7b9afe439a4e9f49</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/scopeguard@1.2.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/scopeguard/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/bluss/scopeguard</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#semver@1.0.26">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>semver</name>
      <version>1.0.26</version>
      <description>Parser and evaluator for Cargo's flavor of Semantic Versioning</description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">56e6fa9c48d24d85fb3de5ad847117517440f6beceb7798af16b4a87d616b8d0</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/semver@1.0.26</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/semver</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/semver</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219">
      <author>Erick Tryzelaar &lt;erick.tryzelaar@gmail.com&gt;, David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>serde</name>
      <version>1.0.219</version>
      <description>A generic serialization/deserialization framework</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5f0e2c6ed6606019b4e29e69dbaba95b11854410e5347d525002456dbbb786b6</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde@1.0.219</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/serde</url>
        </reference>
        <reference type="website">
          <url>https://serde.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/serde-rs/serde</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde_derive@1.0.219">
      <author>Erick Tryzelaar &lt;erick.tryzelaar@gmail.com&gt;, David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>serde_derive</name>
      <version>1.0.219</version>
      <description>Macros 1.1 implementation of #[derive(Serialize, Deserialize)]</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5b0276cf7f2c73365f7157c8123c21cd9a50fbbd844757af28ca1f5925fc2a00</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde_derive@1.0.219</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://serde.rs/derive.html</url>
        </reference>
        <reference type="website">
          <url>https://serde.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/serde-rs/serde</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde_json@1.0.142">
      <author>Erick Tryzelaar &lt;erick.tryzelaar@gmail.com&gt;, David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>serde_json</name>
      <version>1.0.142</version>
      <description>A JSON serialization file format</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">030fedb782600dcbd6f02d479bf0d817ac3bb40d644745b769d6a96bc3afc5a7</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde_json@1.0.142</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/serde_json</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/serde-rs/json</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde_path_to_error@0.1.17">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>serde_path_to_error</name>
      <version>0.1.17</version>
      <description>Path to the element that failed to deserialize</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">59fab13f937fa393d08645bf3a84bdfe86e296747b506ada67bb15f10f218b2a</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde_path_to_error@0.1.17</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/serde_path_to_error</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/path-to-error</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde_urlencoded@0.7.1">
      <author>Anthony Ramine &lt;n.oxyde@gmail.com&gt;</author>
      <name>serde_urlencoded</name>
      <version>0.7.1</version>
      <description>`x-www-form-urlencoded` meets Serde</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">d3491c14715ca2294c4d6a88f15e84739788c1d030eed8c110436aafdaa2f3fd</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde_urlencoded@0.7.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/serde_urlencoded/0.7.1/serde_urlencoded/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/nox/serde_urlencoded</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#serde_yaml@0.9.34+deprecated">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>serde_yaml</name>
      <version>0.9.34+deprecated</version>
      <description>YAML data format for Serde</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6a8b1a1a2ebf674015cc02edccce75287f1a0130d394307b36743c2f5d504b47</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/serde_yaml@0.9.34+deprecated</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/serde_yaml/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/serde-yaml</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#sha2@0.10.9">
      <author>RustCrypto Developers</author>
      <name>sha2</name>
      <version>0.10.9</version>
      <description>Pure Rust implementation of the SHA-2 hash function family including SHA-224, SHA-256, SHA-384, and SHA-512. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/sha2@0.10.9</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/sha2</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/RustCrypto/hashes</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#sharded-slab@0.1.7">
      <author>Eliza Weisman &lt;eliza@buoyant.io&gt;</author>
      <name>sharded-slab</name>
      <version>0.1.7</version>
      <description>A lock-free concurrent slab. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">f40ca3c46823713e0d4209592e8d6e826aa57e928f09752619fc696c499637f6</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/sharded-slab@0.1.7</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/sharded-slab/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/hawkw/sharded-slab</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/hawkw/sharded-slab</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#shlex@1.3.0">
      <author>comex &lt;comexk@gmail.com&gt;, Fenhl &lt;fenhl@fenhl.net&gt;, Adrian Taylor &lt;adetaylor@chromium.org&gt;, Alex Touchet &lt;alextouchet@outlook.com&gt;, Daniel Parks &lt;dp+git@oxidized.org&gt;, Garrett Berg &lt;googberg@gmail.com&gt;</author>
      <name>shlex</name>
      <version>1.3.0</version>
      <description>Split a string into shell words, like Python's shlex.</description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">0fda2ff0d084019ba4d7c6f371c95d8fd75ce3524c3cb8fb653a3023f6323e64</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/shlex@1.3.0</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/comex/rust-shlex</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#signal-hook-registry@1.4.6">
      <author>Michal 'vorner' Vaner &lt;vorner@vorner.cz&gt;, Masaki Hara &lt;ackie.h.gmai@gmail.com&gt;</author>
      <name>signal-hook-registry</name>
      <version>1.4.6</version>
      <description>Backend crate for signal-hook</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b2a4719bff48cee6b39d12c020eeb490953ad2443b7055bd0b21fca26bd8c28b</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/signal-hook-registry@1.4.6</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/signal-hook-registry</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/vorner/signal-hook</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#slab@0.4.10">
      <author>Carl Lerche &lt;me@carllerche.com&gt;</author>
      <name>slab</name>
      <version>0.4.10</version>
      <description>Pre-allocated storage for a uniform data type</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">04dc19736151f35336d325007ac991178d504a119863a2fcb3758cdb5e52c50d</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/slab@0.4.10</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/slab</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#smallstr@0.3.0">
      <author>Murarth &lt;murarth@gmail.com&gt;</author>
      <name>smallstr</name>
      <version>0.3.0</version>
      <description>String-like container based on smallvec</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">63b1aefdf380735ff8ded0b15f31aab05daf1f70216c01c02a12926badd1df9d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/smallstr@0.3.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/smallstr/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/murarth/smallstr</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/murarth/smallstr</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1">
      <author>The Servo Project Developers</author>
      <name>smallvec</name>
      <version>1.15.1</version>
      <description>'Small vector' optimization: store up to a small number of items on the stack</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">67b1b7a3b5fe4f1376887184045fcf45c69e92af734b7aaddc05fb777b6fbd03</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/smallvec@1.15.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/smallvec/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/servo/rust-smallvec</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#socket2@0.6.0">
      <author>Alex Crichton &lt;alex@alexcrichton.com&gt;, Thomas de Zeeuw &lt;thomasdezeeuw@gmail.com&gt;</author>
      <name>socket2</name>
      <version>0.6.0</version>
      <description>Utilities for handling networking sockets with a maximal amount of configuration possible intended. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">233504af464074f9d066d7b5416c5f9b894a5862a6506e306f7b816cdd6f1807</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/socket2@0.6.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/socket2</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rust-lang/socket2</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rust-lang/socket2</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#strsim@0.11.1">
      <author>Danny Guo &lt;danny@dannyguo.com&gt;, maxbachmann &lt;oss@maxbachmann.de&gt;</author>
      <name>strsim</name>
      <version>0.11.1</version>
      <description>Implementations of string similarity metrics. Includes Hamming, Levenshtein, OSA, Damerau-Levenshtein, Jaro, Jaro-Winkler, and Sørensen-Dice. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/strsim@0.11.1</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/strsim/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/rapidfuzz/strsim-rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/rapidfuzz/strsim-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>syn</name>
      <version>2.0.104</version>
      <description>Parser for Rust source code</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">17b6f705963418cdb9927482fa304bc562ece2fdd4f616084c50b7023b435a40</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/syn@2.0.104</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/syn</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/syn</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#sync_wrapper@1.0.2">
      <author>Actyx AG &lt;developer@actyx.io&gt;</author>
      <name>sync_wrapper</name>
      <version>1.0.2</version>
      <description>A tool for enlisting the compiler's help in proving the absence of concurrency</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">0bf256ce5efdfa370213c1dabab5935a12e49f2c58d15e9eac2870d3b4f27263</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/sync_wrapper@1.0.2</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/sync_wrapper</url>
        </reference>
        <reference type="website">
          <url>https://docs.rs/sync_wrapper</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/Actyx/sync_wrapper</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tempfile@3.20.0">
      <author>Steven Allen &lt;steven@stebalien.com&gt;, The Rust Project Developers, Ashley Mannix &lt;ashleymannix@live.com.au&gt;, Jason White &lt;me@jasonwhite.io&gt;</author>
      <name>tempfile</name>
      <version>3.20.0</version>
      <description>A library for managing temporary files and directories.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e8a64e3985349f2441a1a9ef0b853f869006c3855f2cda6862a94d26ebb9d6a1</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/tempfile@3.20.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/tempfile</url>
        </reference>
        <reference type="website">
          <url>https://stebalien.com/projects/tempfile-rs/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/Stebalien/tempfile</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#thiserror-impl@1.0.69">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>thiserror-impl</name>
      <version>1.0.69</version>
      <description>Implementation detail of the `thiserror` crate</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">4fee6c4efc90059e10f81e6d42c60a18f76588c3d74cb83a0b242a2b6c7504c1</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/thiserror-impl@1.0.69</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/dtolnay/thiserror</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#thiserror@1.0.69">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>thiserror</name>
      <version>1.0.69</version>
      <description>derive(Error)</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b6aaf5339b578ea85b50e080feb250a3e8ae8cfcdff9a461c9ec2904bc923f52</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/thiserror@1.0.69</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/thiserror</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/thiserror</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#thread_local@1.1.9">
      <author>Amanieu d'Antras &lt;amanieu@gmail.com&gt;</author>
      <name>thread_local</name>
      <version>1.1.9</version>
      <description>Per-object thread-local storage</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">f60246a4944f24f6e018aa17cdeffb7818b76356965d03b07d6a9886e8962185</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/thread_local@1.1.9</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/thread_local/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/Amanieu/thread_local-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tokio-macros@2.5.0">
      <author>Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tokio-macros</name>
      <version>2.5.0</version>
      <description>Tokio's proc macros. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">6e06d43f1345a3bcd39f6a56dbb7dcab2ba47e68e8ac134855e7e2bdbaf8cab8</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tokio-macros@2.5.0</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tokio</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1">
      <author>Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tokio</name>
      <version>1.47.1</version>
      <description>An event-driven, non-blocking I/O platform for writing asynchronous I/O backed applications. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">89e49afdadebb872d3145a5638b59eb0691ea23e46ca484037cfab3b76b95038</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tokio@1.47.1</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tokio</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tower-http@0.5.2">
      <author>Tower Maintainers &lt;team@tower-rs.com&gt;</author>
      <name>tower-http</name>
      <version>0.5.2</version>
      <description>Tower middleware and utilities for HTTP clients and servers</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1e9cd434a998747dd2c4276bc96ee2e0c7a2eadf3cae88e52be55a05fa9053f5</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tower-http@0.5.2</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/tower-rs/tower-http</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tower-rs/tower-http</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3">
      <author>Tower Maintainers &lt;team@tower-rs.com&gt;</author>
      <name>tower-layer</name>
      <version>0.3.3</version>
      <description>Decorates a `Service` to allow easy composition between `Service`s. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">121c2a6cda46980bb0fcd1647ffaf6cd3fc79a013de288782836f6df9c48780e</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tower-layer@0.3.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/tower-layer/0.3.3</url>
        </reference>
        <reference type="website">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3">
      <author>Tower Maintainers &lt;team@tower-rs.com&gt;</author>
      <name>tower-service</name>
      <version>0.3.3</version>
      <description>Trait representing an asynchronous, request / response based, client or server. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8df9b6e13f2d32c91b9bd719c00d1958837bc7dec474d94952798cc8e69eeec3</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tower-service@0.3.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/tower-service/0.3.3</url>
        </reference>
        <reference type="website">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.4.13">
      <author>Tower Maintainers &lt;team@tower-rs.com&gt;</author>
      <name>tower</name>
      <version>0.4.13</version>
      <description>Tower is a library of modular and reusable components for building robust clients and servers. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b8fa9be0de6cf49e536ce1851f987bd21a43b771b09473c3549a6c853db37c1c</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tower@0.4.13</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/tower/0.4.13</url>
        </reference>
        <reference type="website">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.5.2">
      <author>Tower Maintainers &lt;team@tower-rs.com&gt;</author>
      <name>tower</name>
      <version>0.5.2</version>
      <description>Tower is a library of modular and reusable components for building robust clients and servers. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">d039ad9159c98b70ecfd540b2573b97f7f52c3e8d9f8ad57a24b916a536975f9</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tower@0.5.2</purl>
      <externalReferences>
        <reference type="website">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tower-rs/tower</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tracing-attributes@0.1.30">
      <author>Tokio Contributors &lt;team@tokio.rs&gt;, Eliza Weisman &lt;eliza@buoyant.io&gt;, David Barsky &lt;dbarsky@amazon.com&gt;</author>
      <name>tracing-attributes</name>
      <version>0.1.30</version>
      <description>Procedural macro attributes for automatically instrumenting functions. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">81383ab64e72a7a8b8e13130c49e3dab29def6d0c7d76a03087b3cf71c5c6903</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tracing-attributes@0.1.30</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tracing</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tracing-core@0.1.34">
      <author>Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tracing-core</name>
      <version>0.1.34</version>
      <description>Core primitives for application-level tracing. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">b9d12581f227e93f094d3af2ae690a574abb8a2b9b7a96e7cfe9647b2b617678</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tracing-core@0.1.34</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tracing</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tracing-log@0.2.0">
      <author>Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tracing-log</name>
      <version>0.2.0</version>
      <description>Provides compatibility between `tracing` and the `log` crate. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">ee855f1f400bd0e5c02d150ae5de3840039a3f54b025156404e34c23c03f47c3</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tracing-log@0.2.0</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tracing</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tracing-subscriber@0.3.19">
      <author>Eliza Weisman &lt;eliza@buoyant.io&gt;, David Barsky &lt;me@davidbarsky.com&gt;, Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tracing-subscriber</name>
      <version>0.3.19</version>
      <description>Utilities for implementing and composing `tracing` subscribers. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e8189decb5ac0fa7bc8b96b7cb9b2701d60d48805aca84a238004d665fcc4008</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tracing-subscriber@0.3.19</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tracing</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41">
      <author>Eliza Weisman &lt;eliza@buoyant.io&gt;, Tokio Contributors &lt;team@tokio.rs&gt;</author>
      <name>tracing</name>
      <version>0.1.41</version>
      <description>Application-level tracing for Rust. </description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">784e0ac535deb450455cbfa28a6f0df145ea1bb7ae51b821cf5e7927fdcfbdd0</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/tracing@0.1.41</purl>
      <externalReferences>
        <reference type="website">
          <url>https://tokio.rs</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/tokio-rs/tracing</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#typenum@1.18.0">
      <author>Paho Lurie-Gregg &lt;paho@paholg.com&gt;, Andre Bogus &lt;bogusandre@gmail.com&gt;</author>
      <name>typenum</name>
      <version>1.18.0</version>
      <description>Typenum is a Rust library for type-level numbers evaluated at     compile time. It currently supports bits, unsigned integers, and signed     integers. It also provides a type-level array of type-level numbers, but its     implementation is incomplete.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1dccffe3ce07af9386bfd29e80c0ab1a8205a2fc34e4bcd40364df902cfa8f3f</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/typenum@1.18.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/typenum</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/paholg/typenum</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#unicode-ident@1.0.18">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>unicode-ident</name>
      <version>1.0.18</version>
      <description>Determine whether characters have the XID_Start or XID_Continue properties according to Unicode Standard Annex #31</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">5a5f39404a5da50712a4c1eecf25e90dd62b613502b7e925fd4e4d19b5c96512</hash>
      </hashes>
      <licenses>
        <expression>(MIT OR Apache-2.0) AND Unicode-3.0</expression>
      </licenses>
      <purl>pkg:cargo/unicode-ident@1.0.18</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/unicode-ident</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/unicode-ident</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#unsafe-libyaml@0.2.11">
      <author>David Tolnay &lt;dtolnay@gmail.com&gt;</author>
      <name>unsafe-libyaml</name>
      <version>0.2.11</version>
      <description>libyaml transpiled to rust by c2rust</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">673aac59facbab8a9007c7f6108d11f63b603f7cabff99fabf650fea5c32b861</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/unsafe-libyaml@0.2.11</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/unsafe-libyaml</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/dtolnay/unsafe-libyaml</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#utf8parse@0.2.2">
      <author>Joe Wilm &lt;joe@jwilm.com&gt;, Christian Duerr &lt;contact@christianduerr.com&gt;</author>
      <name>utf8parse</name>
      <version>0.2.2</version>
      <description>Table-driven UTF-8 parser</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">06abde3611657adf66d383f00b093d7faecc7fa57071cce2578660c9f1010821</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/utf8parse@0.2.2</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/utf8parse/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/alacritty/vte</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#uuid@1.17.0">
      <author>Ashley Mannix&lt;ashleymannix@live.com.au&gt;, Dylan DPC&lt;dylan.dpc@gmail.com&gt;, Hunar Roop Kahlon&lt;hunar.roop@gmail.com&gt;</author>
      <name>uuid</name>
      <version>1.17.0</version>
      <description>A library to generate and parse UUIDs.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">3cf4199d1e5d15ddd86a694e4d0dffa9c323ce759fea589f00fef9d81cc1931d</hash>
      </hashes>
      <licenses>
        <expression>Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/uuid@1.17.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/uuid</url>
        </reference>
        <reference type="website">
          <url>https://github.com/uuid-rs/uuid</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/uuid-rs/uuid</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#version_check@0.9.5">
      <author>Sergio Benitez &lt;sb@sergio.bz&gt;</author>
      <name>version_check</name>
      <version>0.9.5</version>
      <description>Tiny crate to check the version of the installed/running rustc.</description>
      <scope>excluded</scope>
      <hashes>
        <hash alg="SHA-256">0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/version_check@0.9.5</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/version_check/</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/SergioBenitez/version_check</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#walkdir@2.5.0">
      <author>Andrew Gallant &lt;jamslam@gmail.com&gt;</author>
      <name>walkdir</name>
      <version>2.5.0</version>
      <description>Recursively walk a directory.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">29790946404f91d9c5d06f9874efddea1dc06c5efe94541a7d6863108e3a5e4b</hash>
      </hashes>
      <licenses>
        <expression>Unlicense OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/walkdir@2.5.0</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/walkdir/</url>
        </reference>
        <reference type="website">
          <url>https://github.com/BurntSushi/walkdir</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/BurntSushi/walkdir</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#zerocopy@0.8.26">
      <author>Joshua Liebow-Feeser &lt;joshlf@google.com&gt;, Jack Wrenn &lt;jswrenn@amazon.com&gt;</author>
      <name>zerocopy</name>
      <version>0.8.26</version>
      <description>Zerocopy makes zero-cost memory manipulation effortless. We write "unsafe" so you don't have to.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">1039dd0d3c310cf05de012d8a39ff557cb0d23087fd44cad61df08fc31907a2f</hash>
      </hashes>
      <licenses>
        <expression>BSD-2-Clause OR Apache-2.0 OR MIT</expression>
      </licenses>
      <purl>pkg:cargo/zerocopy@0.8.26</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/google/zerocopy</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#zstd-safe@7.2.4">
      <author>Alexandre Bury &lt;alexandre.bury@gmail.com&gt;</author>
      <name>zstd-safe</name>
      <version>7.2.4</version>
      <description>Safe low-level bindings for the zstd compression library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">8f49c4d5f0abb602a93fb8736af2a4f4dd9512e36f7f570d66e65ff867ed3b9d</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/zstd-safe@7.2.4</purl>
      <externalReferences>
        <reference type="vcs">
          <url>https://github.com/gyscos/zstd-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#zstd-sys@2.0.15+zstd.1.5.7">
      <author>Alexandre Bury &lt;alexandre.bury@gmail.com&gt;</author>
      <name>zstd-sys</name>
      <version>2.0.15+zstd.1.5.7</version>
      <description>Low-level bindings for the zstd compression library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">eb81183ddd97d0c74cedf1d50d85c8d08c1b8b68ee863bdee9e706eedba1a237</hash>
      </hashes>
      <licenses>
        <expression>MIT OR Apache-2.0</expression>
      </licenses>
      <purl>pkg:cargo/zstd-sys@2.0.15+zstd.1.5.7</purl>
      <externalReferences>
        <reference type="other">
          <url>zstd</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/gyscos/zstd-rs</url>
        </reference>
      </externalReferences>
    </component>
    <component type="library" bom-ref="registry+https://github.com/rust-lang/crates.io-index#zstd@0.13.3">
      <author>Alexandre Bury &lt;alexandre.bury@gmail.com&gt;</author>
      <name>zstd</name>
      <version>0.13.3</version>
      <description>Binding for the zstd compression library.</description>
      <scope>required</scope>
      <hashes>
        <hash alg="SHA-256">e91ee311a569c327171651566e07972200e76fcfe2242a4fa446149a3881c08a</hash>
      </hashes>
      <licenses>
        <expression>MIT</expression>
      </licenses>
      <purl>pkg:cargo/zstd@0.13.3</purl>
      <externalReferences>
        <reference type="documentation">
          <url>https://docs.rs/zstd</url>
        </reference>
        <reference type="vcs">
          <url>https://github.com/gyscos/zstd-rs</url>
        </reference>
      </externalReferences>
    </component>
  </components>
  <dependencies>
    <dependency ref="path+file:///Users/jayminwest/Projects/kota-db#kotadb@0.1.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anyhow@1.0.98" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#async-trait@0.1.88" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#axum@0.7.9" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bincode@1.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#chrono@0.4.41" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap@4.5.42" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crc32c@0.6.8" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#dashmap@6.1.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fastrand@2.3.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper@1.6.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#indexmap@2.10.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lz4@1.28.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#md5@0.7.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memmap2@0.9.7" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#notify@6.1.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand@0.8.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rmp-serde@1.3.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#roaring@0.10.12" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_json@1.0.142" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_yaml@0.9.34+deprecated" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sha2@0.10.9" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallstr@0.3.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tempfile@3.20.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thiserror@1.0.69" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.4.13" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-http@0.5.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-subscriber@0.3.19" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#uuid@1.17.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd@0.13.3" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#aho-corasick@1.1.3">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstream@0.6.20">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle@1.0.11" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-parse@0.2.7" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-query@1.1.4" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#colorchoice@1.0.4" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#is_terminal_polyfill@1.70.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#utf8parse@0.2.2" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-parse@0.2.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#utf8parse@0.2.2" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle-query@1.1.4" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle@1.0.11" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anyhow@1.0.98" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#async-trait@0.1.88">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#autocfg@1.5.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#axum-core@0.4.5">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#async-trait@0.1.88" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body-util@0.1.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#mime@0.3.17" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustversion@1.0.21" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sync_wrapper@1.0.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#axum@0.7.9">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#async-trait@0.1.88" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#axum-core@0.4.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body-util@0.1.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper@1.6.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper-util@0.1.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#matchit@0.7.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#mime@0.3.17" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#percent-encoding@2.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustversion@1.0.21" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_json@1.0.142" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_path_to_error@0.1.17" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_urlencoded@0.7.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sync_wrapper@1.0.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.5.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bincode@1.3.3">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bitflags@2.9.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#block-buffer@0.10.4">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#generic-array@0.14.7" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytemuck@1.23.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#byteorder@1.5.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cc@1.2.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#jobserver@0.1.33" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#shlex@1.3.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#chrono@0.4.41">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#iana-time-zone@0.1.63" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#num-traits@0.2.19" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap@4.5.42">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_builder@4.5.42" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_derive@4.5.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_builder@4.5.42">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstream@0.6.20" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#anstyle@1.0.11" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_lex@0.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#strsim@0.11.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_derive@4.5.41">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#heck@0.5.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#clap_lex@0.7.5" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#colorchoice@1.0.4" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#core-foundation-sys@0.8.7" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cpufeatures@0.2.17">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crc32c@0.6.8">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustc_version@0.4.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-channel@0.5.15">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-utils@0.8.21" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-utils@0.8.21" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crypto-common@0.1.6">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#generic-array@0.14.7" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#typenum@1.18.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#dashmap@6.1.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-utils@0.8.21" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.14.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lock_api@0.4.13" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot_core@0.9.11" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#digest@0.10.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#block-buffer@0.10.4" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crypto-common@0.1.6" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#equivalent@1.0.2" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#errno@0.3.13">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fastrand@2.3.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#filetime@0.2.25">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fnv@1.0.7" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#form_urlencoded@1.2.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#percent-encoding@2.3.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fsevent-sys@4.1.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-channel@0.3.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-sink@0.3.31" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-executor@0.3.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-task@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-io@0.3.31" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-macro@0.3.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-sink@0.3.31" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-task@0.3.31" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-channel@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-io@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-macro@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-sink@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-task@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-utils@0.1.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#slab@0.4.10" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures@0.3.31">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-channel@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-executor@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-io@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-sink@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-task@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#generic-array@0.14.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#typenum@1.18.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#version_check@0.9.5" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.2.16">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.3.3">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.14.5" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.15.4" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#heck@0.5.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body-util@0.1.3">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fnv@1.0.7" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#httparse@1.10.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#httpdate@1.0.3" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper-util@0.1.16">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper@1.6.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hyper@1.6.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-channel@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#httparse@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#httpdate@1.0.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#iana-time-zone@0.1.63">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#core-foundation-sys@0.8.7" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#indexmap@2.10.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#equivalent@1.0.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#hashbrown@0.15.4" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#is_terminal_polyfill@1.70.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#jobserver@0.1.33">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lazy_static@1.5.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lock_api@0.4.13">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#autocfg@1.5.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#scopeguard@1.2.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#log@0.4.27" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lz4-sys@1.11.1+lz4-1.10.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cc@1.2.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lz4@1.28.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lz4-sys@1.11.1+lz4-1.10.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#matchers@0.1.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.1.10" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#matchit@0.7.3" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#md5@0.7.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memmap2@0.9.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#mime@0.3.17" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#mio@1.0.4">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#notify@6.1.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bitflags@2.9.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#crossbeam-channel@0.5.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#filetime@0.2.25" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fsevent-sys@4.1.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#log@0.4.27" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#walkdir@2.5.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#nu-ansi-term@0.46.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#overload@0.1.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#num-traits@0.2.19">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#autocfg@1.5.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#overload@0.1.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot@0.12.4">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lock_api@0.4.13" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot_core@0.9.11" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot_core@0.9.11">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#paste@1.0.15" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#percent-encoding@2.3.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-internal@1.1.10">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project@1.1.10">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-internal@1.1.10" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-utils@0.1.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pkg-config@0.3.32" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ppv-lite86@0.2.21">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zerocopy@0.8.26" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#unicode-ident@1.0.18" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand@0.8.5">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand_chacha@0.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand_core@0.6.4" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand_chacha@0.3.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ppv-lite86@0.2.21" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand_core@0.6.4" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rand_core@0.6.4">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.2.16" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.1.10">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.6.29" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.4.9">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#aho-corasick@1.1.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.8.5" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.6.29" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.8.5" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex@1.11.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#aho-corasick@1.1.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-automata@0.4.9" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex-syntax@0.8.5" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rmp-serde@1.3.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#byteorder@1.5.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rmp@0.8.14" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rmp@0.8.14">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#byteorder@1.5.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#num-traits@0.2.19" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#paste@1.0.15" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#roaring@0.10.12">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytemuck@1.23.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#byteorder@1.5.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustc_version@0.4.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#semver@1.0.26" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustix@1.0.8">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bitflags@2.9.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#errno@0.3.13" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustversion@1.0.21" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ryu@1.0.20" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#same-file@1.0.6" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#scopeguard@1.2.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#semver@1.0.26" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_derive@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_derive@1.0.219">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_json@1.0.142">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#memchr@2.7.5" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ryu@1.0.20" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_path_to_error@0.1.17">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_urlencoded@0.7.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#form_urlencoded@1.2.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ryu@1.0.20" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde_yaml@0.9.34+deprecated">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#indexmap@2.10.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#itoa@1.0.15" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#ryu@1.0.20" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#unsafe-libyaml@0.2.11" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sha2@0.10.9">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cpufeatures@0.2.17" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#digest@0.10.7" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sharded-slab@0.1.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#lazy_static@1.5.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#shlex@1.3.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#signal-hook-registry@1.4.6">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#slab@0.4.10" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallstr@0.3.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#socket2@0.6.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#strsim@0.11.1" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#unicode-ident@1.0.18" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sync_wrapper@1.0.2" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tempfile@3.20.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#fastrand@2.3.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#rustix@1.0.8" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thiserror-impl@1.0.69">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thiserror@1.0.69">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thiserror-impl@1.0.69" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thread_local@1.1.9">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.1" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio-macros@2.5.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#libc@0.2.174" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#mio@1.0.4" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#parking_lot@0.12.4" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#signal-hook-registry@1.4.6" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#socket2@0.6.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio-macros@2.5.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-http@0.5.2">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bitflags@2.9.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#bytes@1.10.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http@1.3.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body@1.0.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#http-body-util@0.1.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.4.13">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project@1.1.10" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower@0.5.2">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-core@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#futures-util@0.3.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sync_wrapper@1.0.2" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tokio@1.47.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-layer@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tower-service@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-attributes@0.1.30">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.95" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#quote@1.0.40" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#syn@2.0.104" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-core@0.1.34">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-log@0.2.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#log@0.4.27" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-core@0.1.34" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-subscriber@0.3.19">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#matchers@0.1.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#nu-ansi-term@0.46.0" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#once_cell@1.21.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#regex@1.11.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#sharded-slab@0.1.7" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#smallvec@1.15.1" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#thread_local@1.1.9" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-core@0.1.34" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-log@0.2.0" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing@0.1.41">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#log@0.4.27" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pin-project-lite@0.2.16" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-attributes@0.1.30" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#tracing-core@0.1.34" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#typenum@1.18.0" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#unicode-ident@1.0.18" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#unsafe-libyaml@0.2.11" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#utf8parse@0.2.2" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#uuid@1.17.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#getrandom@0.3.3" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#serde@1.0.219" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#version_check@0.9.5" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#walkdir@2.5.0">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#same-file@1.0.6" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zerocopy@0.8.26" />
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd-safe@7.2.4">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd-sys@2.0.15+zstd.1.5.7" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd-sys@2.0.15+zstd.1.5.7">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#cc@1.2.31" />
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#pkg-config@0.3.32" />
    </dependency>
    <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd@0.13.3">
      <dependency ref="registry+https://github.com/rust-lang/crates.io-index#zstd-safe@7.2.4" />
    </dependency>
  </dependencies>
</bom>