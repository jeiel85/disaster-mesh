package org.disastermesh.android.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.io.FileOutputStream
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class MasterKeyUnavailableException(message: String, cause: Throwable? = null) :
    Exception(message, cause)

/** Android owns only the non-exportable wrapping key and passes the unwrapped DB key once to Rust. */
class MasterKeyManager(private val context: Context) {
    private val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
    private val wrappedFile: File = File(context.noBackupFilesDir, WRAPPED_FILE)

    @Synchronized
    @Throws(MasterKeyUnavailableException::class)
    fun loadOrCreate(): ByteArray {
        val aliasExists = keyStore.containsAlias(ALIAS)
        val blobExists = wrappedFile.exists()
        if (aliasExists xor blobExists) {
            throw MasterKeyUnavailableException(
                "Local encryption state is incomplete; explicit recovery or reset is required",
            )
        }
        return if (!aliasExists) create() else unwrap()
    }

    private fun create(): ByteArray {
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        val wrappingKey = generator.generateKey()
        val masterKey = ByteArray(32).also(SecureRandom()::nextBytes)
        try {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, wrappingKey)
            val ciphertext = cipher.doFinal(masterKey)
            val envelope = byteArrayOf(VERSION) + cipher.iv + ciphertext
            val temporary = File(wrappedFile.parentFile, "$WRAPPED_FILE.tmp")
            FileOutputStream(temporary).use { output ->
                output.write(envelope)
                output.fd.sync()
            }
            if (!temporary.renameTo(wrappedFile)) {
                temporary.delete()
                keyStore.deleteEntry(ALIAS)
                throw MasterKeyUnavailableException("Unable to persist wrapped local key")
            }
            return masterKey
        } catch (error: MasterKeyUnavailableException) {
            masterKey.fill(0)
            throw error
        } catch (error: Exception) {
            masterKey.fill(0)
            keyStore.deleteEntry(ALIAS)
            throw MasterKeyUnavailableException("Unable to create local encryption key", error)
        }
    }

    private fun unwrap(): ByteArray = try {
        val envelope = wrappedFile.readBytes()
        if (envelope.size != ENVELOPE_BYTES || envelope[0] != VERSION) {
            throw MasterKeyUnavailableException("Wrapped local key is corrupt")
        }
        val wrappingKey = keyStore.getKey(ALIAS, null) as? SecretKey
            ?: throw MasterKeyUnavailableException("Wrapping key is unavailable")
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(
            Cipher.DECRYPT_MODE,
            wrappingKey,
            GCMParameterSpec(128, envelope.copyOfRange(1, 13)),
        )
        cipher.doFinal(envelope.copyOfRange(13, envelope.size)).also { plaintext ->
            if (plaintext.size != 32) {
                plaintext.fill(0)
                throw MasterKeyUnavailableException("Unwrapped local key has invalid length")
            }
        }
    } catch (error: MasterKeyUnavailableException) {
        throw error
    } catch (error: Exception) {
        throw MasterKeyUnavailableException(
            "Local encryption key cannot be unwrapped; automatic identity reset is prohibited",
            error,
        )
    }

    companion object {
        private const val KEYSTORE = "AndroidKeyStore"
        private const val ALIAS = "dm_local_wrap_v1"
        private const val WRAPPED_FILE = "dm_master_key_v1.bin"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val VERSION: Byte = 1
        private const val ENVELOPE_BYTES = 1 + 12 + 32 + 16
    }
}
