import org.tomlj.Toml
import pers.zhc.gradle.plugins.ndk.ConfigParser
import pers.zhc.gradle.plugins.ndk.GradleExtensionConfigConverters
import pers.zhc.gradle.plugins.ndk.rust.RustBuildPlugin
import pers.zhc.gradle.plugins.ndk.rust.RustBuildPlugin.RustBuildPluginExtension
import java.util.*

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "pers.zhc.android.myapplication"
    compileSdk = 35

    defaultConfig {
        applicationId = "pers.zhc.android.myapplication"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        viewBinding = true
    }

    sourceSets {
        val sets = asMap
        sets["main"]!!.apply {
            jniLibs.srcDirs("jniLibs")
        }
    }

    signingConfigs {
        val configs = asMap
        configs["debug"]!!.apply {
            storeFile = file("release.keystore")
            storePassword = "123456"
            keyAlias = "alias"
            keyPassword = "123456"
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)
    implementation(libs.androidx.constraintlayout)
    implementation(libs.androidx.navigation.fragment.ktx)
    implementation(libs.androidx.navigation.ui.ktx)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}

apply<RustBuildPlugin>()

val appProject = project
val localConfig = getAndroidLocalConfig()

val configTomlFile = File(rootDir, "config.toml")

if (!configTomlFile.exists()) {
    throw GradleException("File required: $configTomlFile")
}

val parsed = ConfigParser.parse(configTomlFile)

val configToml = Toml.parse(configTomlFile.reader())!!

configToml.errors().forEach {
    throw GradleException("`config.toml` parsing error: $it")
}

configure<RustBuildPluginExtension> {
    srcDir.set("$projectDir/src/main/rust")
    ndkDir.set(localConfig.ndkDir)
    targets.set(GradleExtensionConfigConverters.targetsToMap(parsed.ndk.targets))
    buildType.set(parsed.ndk.buildType.name)
    outputDir.set(File(appProject.projectDir, "jniLibs").also { it.mkdirs() }.path)
}

val compileRustTask = project.tasks.getByName("compileRust")
val compileJniTask = task("compileJni") {
    dependsOn(compileRustTask)
}
appProject.tasks.getByName("preBuild").dependsOn(compileJniTask)

// ================== Utility functions ==================
data class AndroidLocalConfig(
    val sdkDir: String,
    val ndkDir: String,
)

fun getAndroidLocalConfig(): AndroidLocalConfig {
    val props = Properties()
    val localPropertiesFile = File(rootDir, "local.properties")

    if (!localPropertiesFile.exists()) {
        throw GradleException("Missing 'local.properties' file in project root.")
    }

    localPropertiesFile.inputStream().use { props.load(it) }

    // Helper to fetch and validate properties
    fun getRequiredProperty(key: String): String {
        val value = props.getProperty(key)
        if (value.isNullOrBlank()) {
            throw GradleException("Property '$key' is missing in local.properties. Please provide it.")
        }
        return value
    }

    return AndroidLocalConfig(
        sdkDir = getRequiredProperty("sdk.dir"),
        ndkDir = getRequiredProperty("ndk.dir"),
    )
}
