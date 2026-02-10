// Top-level build file where you can add configuration options common to all sub-projects/modules.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
}

buildscript {
    dependencies {
        classpath("com.github.bczhc:android-native-build-plugin:e95ac75536")
        classpath("com.github.bczhc:android-native-build-plugin-config-parser:f4eee68fd2")
        classpath("com.github.bczhc:android-target-defs") {
            version {
                strictly("ac1ea2f9fc")
            }
        }
    }
}
