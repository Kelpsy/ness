// const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const { resolve, join } = require("path");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const CopyPlugin = require("copy-webpack-plugin");

const fontawesomePath = require.resolve("@fortawesome/fontawesome-free");

const src = resolve(__dirname, "src");
const dist = resolve(__dirname, "dist");

const mode = "development";
const sourceMap = mode === "development";
const optimize = mode === "production";

const plugins = [
    // new WasmPackPlugin({
    //     crateDirectory: resolve(__dirname, "crate"),
    //     watchDirectories: [
    //         resolve(__dirname, "../../core"),
    //     ],
    //     forceMode: "production",
    // }),
    new CleanWebpackPlugin(),
    new MiniCssExtractPlugin(),
    new CopyPlugin({
        patterns: [
            resolve(src, "index.html"),
            resolve(src, "test.png"),
            { from: join(fontawesomePath, "../../css"), to: "fontawesome/css" },
            { from: join(fontawesomePath, "../../webfonts"), to: "fontawesome/webfonts" },
        ],
    }),
];

if (optimize) {
    plugins.push(
        new (require("optimize-css-assets-webpack-plugin"))({
            cssProcessorPluginOptions: {
                preset: ["default", { discardComments: true }],
            },
        }),
    );
} else {
    plugins.push(new (require("fork-ts-checker-webpack-plugin"))());
}

module.exports = {
    context: resolve(__dirname),
    entry: [
        resolve(src, "main.less"),
        resolve(src, "main.ts"),
    ],
    devServer: {
        static: [dist],
        compress: true,
        host: "0.0.0.0",
        port: 2626,
    },
    devtool: sourceMap ? "eval-source-map" : undefined,
    plugins,
    module: {
        rules: [
            {
                test: /\.less$/i,
                use: [
                    MiniCssExtractPlugin.loader,
                    {
                        loader: "css-loader",
                        options: {
                            sourceMap,
                        },
                    },
                    {
                        loader: "less-loader",
                        options: {
                            sourceMap,
                        },
                    },
                ],
            },
            {
                test: /\.css$/i,
                use: [
                    MiniCssExtractPlugin.loader,
                    {
                        loader: "css-loader",
                        options: {
                            sourceMap,
                        },
                    },
                ],
            },
            {
                test: /\.(eot|svg|ttf|woff|woff2|png|map)$/i,
                type: "asset/resource",
                generator: {
                    filename: "[name].[ext]",
                },
            },
            {
                test: /\.tsx?$/i,
                use: {
                    loader: "ts-loader",
                    options: {
                        transpileOnly: !optimize,
                        configFile: resolve(__dirname, "tsconfig.json"),
                        compilerOptions: {
                            sourceMap,
                        },
                    },
                },
            },
        ],
    },
    resolve: {
        extensions: [".ts", ".tsx", ".js", ".json"],
    },
    output: {
        filename: "[name].bundle.js",
        path: dist,
    },
    experiments: {
        asyncWebAssembly: true,
    },
    optimization: optimize ? {
        minimize: true,
        minimizer: [new (require("css-minimizer-webpack-plugin"))(), "..."],
    } : {},
    watchOptions: {
        ignored: ["**/node_modules", "dist"],
    },
    mode,
};
